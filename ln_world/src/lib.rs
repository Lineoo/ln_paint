use std::{
    any::{Any, TypeId, type_name},
    borrow::Cow,
    cell::RefCell,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::mpsc::{Receiver, Sender, channel},
};

use hashbrown::{HashMap, HashSet};
use indexmap::IndexMap;
use smallvec::SmallVec;

// Definition //

/// A shared form of objects in the [`World`].
pub trait Element: Any + Sized + Send {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        let _ = (world, this);
    }

    fn when_remove(&mut self, world: &World, this: Handle<Self>) {
        let _ = (world, this);
    }
}

/// A way to setup in the [`World`].
pub trait Descriptor {
    type Target;
    fn when_build(self, world: &World) -> Self::Target;
}

// Handle //

/// Represent an element in the [`World`]. It may not be valid.
pub struct Handle<T: Element>(u32, u32, PhantomData<fn() -> T>);

impl<T: Element> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Element> Copy for Handle<T> {}

impl<T: Element> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl<T: Element> Eq for Handle<T> {}

impl<T: Element> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
        self.1.hash(state);
    }
}

impl<T: Element> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Handle<{}>({}v{})",
            shorter_type(type_name::<T>()),
            self.0,
            self.1
        )
    }
}

impl<T: Element> fmt::Display for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}v{}", self.0, self.1)
    }
}

/// Represent an untyped element in the [`World`]. It may not be valid.
pub struct HandleAny(u32, u32);

impl fmt::Debug for HandleAny {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Handle({}v{})", self.0, self.1)
    }
}

impl fmt::Display for HandleAny {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}v{}", self.0, self.1)
    }
}

impl Clone for HandleAny {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for HandleAny {}

impl PartialEq for HandleAny {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl Eq for HandleAny {}

impl Hash for HandleAny {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
        self.1.hash(state);
    }
}

impl HandleAny {
    const fn cast<U: Element>(self) -> Handle<U> {
        Handle(self.0, self.1, PhantomData)
    }
}

pub trait HandleGeneric: Send + Sync + Clone + Copy + PartialEq + Eq + 'static {
    fn untyped(self) -> HandleAny;
}

impl<T: Element> HandleGeneric for Handle<T> {
    fn untyped(self) -> HandleAny {
        HandleAny(self.0, self.1)
    }
}

impl HandleGeneric for HandleAny {
    fn untyped(self) -> HandleAny {
        HandleAny(self.0, self.1)
    }
}

/// Handle with debug information.
#[derive(Clone)]
pub struct HandleInfo {
    handle: HandleAny,
    class: &'static str,
    trace: Cow<'static, str>,
}

impl fmt::Debug for HandleInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Handle[{}]<{}>({}v{})",
            self.trace,
            shorter_type(self.class),
            self.handle.0,
            self.handle.1,
        )
    }
}

impl fmt::Display for HandleInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.handle.fmt(f)
    }
}

fn shorter_type(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut cnt = 0;
    let mut pattern = String::new();

    for ch in s.chars() {
        if ch == ':' {
            cnt += 1;
            if cnt == 2 {
                pattern.clear();
                cnt = 0;
            }
        } else {
            if cnt == 1 {
                if !pattern.is_empty() {
                    out.push_str(&pattern);
                    pattern.clear();
                }
                out.push(':');
                cnt = 0;
            }

            if ch.is_alphanumeric() || ch == '_' || ch == '#' {
                pattern.push(ch);
            } else {
                if !pattern.is_empty() {
                    out.push_str(&pattern);
                    pattern.clear();
                }
                out.push(ch);
            }
        }
    }

    if cnt == 1 {
        if !pattern.is_empty() {
            out.push_str(&pattern);
            pattern.clear();
        }
        out.push(':');
    } else if !pattern.is_empty() {
        out.push_str(&pattern);
    }

    out
}

// World Management //

// Center of multiple accesses in world, which also prevents constructional changes
pub struct World {
    elem_gen: RefCell<Vec<u32>>,
    elem_free: RefCell<Vec<u32>>,

    indices: HashMap<HandleAny, HandleIndex>,
    storages: HashMap<TypeId, Box<dyn StorageGeneral>>,
    cache: RefCell<HashMap<TypeId, SmallVec<[usize; 1]>>>,

    occupied: RefCell<HashMap<HandleAny, isize>>,
    inserted: RefCell<HashSet<HandleAny>>,
    removed: RefCell<HashSet<HandleAny>>,

    scope: RefCell<WorldScope>,
    dependencies: RefCell<Dependencies>,

    queue: Receiver<WorldCommand>,
    commander: Sender<WorldCommand>,
}

#[derive(Clone)]
struct WorldScope {
    label: Cow<'static, str>,
    autodeps: Option<HandleAny>,
}

struct HandleIndex {
    tid: TypeId,
    trace: Cow<'static, str>,
}

struct Storage<T: Element>(IndexMap<HandleAny, T>);

trait StorageGeneral: Any {
    fn name(&self) -> &'static str;
    fn remove(&mut self, handle: HandleAny);
    fn when_remove(&mut self, world: &World, handle: HandleAny);
}

impl<T: Element> StorageGeneral for Storage<T> {
    fn name(&self) -> &'static str {
        type_name::<T>()
    }

    fn remove(&mut self, handle: HandleAny) {
        self.0.swap_remove(&handle);
    }

    fn when_remove(&mut self, world: &World, handle: HandleAny) {
        let elem = self.0.get_mut(&handle).unwrap();
        T::when_remove(elem, world, handle.cast());
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("{0:?} was just inserted, not flushed yet")]
    JustInserted(HandleInfo),

    #[error("{0:?} was just removed, not flushed yet")]
    JustRemoved(HandleInfo),

    #[error("{0:?} does not exist")]
    InvalidHandle(HandleInfo),

    #[error("{0:?} try to depend on {1:?}, which does not exist")]
    ToxicDependency(HandleInfo, HandleInfo),

    #[error("{0:?} has wrong type")]
    UnmatchedType(HandleInfo),

    #[error("{0:?} is mutably borrowed")]
    Unavailable(HandleInfo),

    #[error("{0:?} is borrowed")]
    UnavailableMut(HandleInfo),

    #[error("{0} is not singleton (0 in total)")]
    SingletonNoSuch(&'static str),

    #[error("{0} is not singleton ({1} in total)")]
    SingletonTooMany(&'static str, usize),

    #[error("{0} may be singleton, but not flushed")]
    SingletonCorrupted(&'static str),
}

impl World {
    pub fn new() -> Self {
        let (commander, queue) = channel();

        World {
            elem_gen: RefCell::default(),
            elem_free: RefCell::default(),
            indices: HashMap::new(),
            storages: HashMap::new(),
            cache: RefCell::default(),
            occupied: RefCell::default(),
            inserted: RefCell::default(),
            removed: RefCell::default(),
            scope: RefCell::new(WorldScope {
                label: "init".into(),
                autodeps: None,
            }),
            dependencies: RefCell::default(),
            queue,
            commander,
        }
    }

    /// Will access data from world to build target object.
    pub fn build<B: Descriptor>(&self, descriptor: B) -> B::Target {
        descriptor.when_build(self)
    }

    /// Allocate a slot for a new element, reusing a freed index if available and bumping its
    /// generation on reuse so stale handles never alias fresh ones.
    fn allocate(&self) -> (u32, u32) {
        let mut free = self.elem_free.borrow_mut();
        let mut generation = self.elem_gen.borrow_mut();

        if let Some(index) = free.pop() {
            (index, generation[index as usize])
        } else {
            let index = generation.len() as u32;
            generation.push(0);
            (index, 0)
        }
    }

    /// Free a slot, bumping its generation so any outstanding handle to it becomes invalid.
    fn deallocate(&self, index: u32) {
        let mut generation = self.elem_gen.borrow_mut();
        generation[index as usize] += 1;
        self.elem_free.borrow_mut().push(index);
    }

    /// Due to limit of cell, the inserted element cannot be fetched until `flush` is called.
    /// Meanwhile, handle-based ops, like `observer` or `dependency`, can still be used normally.
    pub fn insert<T: Element>(&self, element: T) -> Handle<T> {
        self.insert_with(move |_| element)
    }

    /// Due to limit of cell, the inserted element cannot be fetched until `flush` is called.
    /// Meanwhile, handle-based ops, like `observer` or `dependency`, can still be used normally.
    pub fn insert_with<T: Element>(
        &self,
        element: impl FnOnce(&World) -> T + Send + 'static,
    ) -> Handle<T> {
        // assign estimate handle
        let (index, generation) = self.allocate();
        let handle = Handle::<T>(index, generation, PhantomData);

        // write immediate record
        let mut inserted = self.inserted.borrow_mut();
        inserted.insert(handle.untyped());

        let scope = self.scope.borrow();
        if let Some(parent) = scope.autodeps {
            self.dependency(handle, parent);
        }

        // delay execution
        let trace = self.label();
        self.queue(move |world| {
            world.scope(format!("{} {{{:?}}}", trace, handle));

            // initialize element
            let element = element(world);

            world.scope(format!("{} {:?}", trace, handle));

            // get type table ready
            let storage = world.storages.entry(TypeId::of::<T>()).or_insert_with(|| {
                log::trace!("register elements: {}", type_name::<T>());
                Box::new(Storage::<T>(IndexMap::new()))
            });

            // push into storage
            let storage = (storage.as_mut() as &mut dyn Any)
                .downcast_mut::<Storage<T>>()
                .unwrap();
            storage.0.insert(handle.untyped(), element);

            // update typetable
            world.indices.insert(
                handle.untyped(),
                HandleIndex {
                    tid: TypeId::of::<T>(),
                    trace,
                },
            );
            world.inserted.get_mut().remove(&handle.untyped());

            // junk cache
            world.cache.get_mut().remove(&TypeId::of::<T>());

            // when_insert
            let mut element = world.fetch_mut(handle);
            element.when_insert(world, handle);
        });

        handle
    }

    pub fn remove(&self, handle: impl HandleGeneric) -> usize {
        self.unwrap(self.try_remove(handle))
    }

    pub fn remove_with<T: Element>(
        &self,
        handle: Handle<T>,
        recycle: impl FnOnce(T, &World) + Send + 'static,
    ) -> usize {
        self.unwrap(self.try_remove_with(handle, recycle))
    }

    /// Cell-mode removal cannot access the element immediately so we can't return the owned value of removed element.
    pub fn try_remove(&self, handle: impl HandleGeneric) -> Result<usize, WorldError> {
        let handle = handle.untyped();
        let info = self.info(handle);

        self.available_mut(handle)?;
        let mut cnt = 1;

        let trace = self.scope.borrow().clone();
        self.scope(format!("{} ~{:?}", trace.label, info));

        // cleanup parents' dependencies
        let mut dependencies = self.dependencies.borrow_mut();
        if let Some(deps) = dependencies.0.get_mut(&handle.untyped()) {
            for parent in std::mem::take(&mut deps.parents) {
                let parent_deps = dependencies.0.get_mut(&parent).unwrap();
                parent_deps
                    .children
                    .retain(|child| *child != handle.untyped());
            }
        }
        drop(dependencies);

        // remove children
        loop {
            let mut dependencies = self.dependencies.borrow_mut();
            let Some(dep) = dependencies.0.get(&handle.untyped()) else {
                break;
            };

            let Some(&child) = dep.children.last() else {
                dependencies.0.remove(&handle.untyped());
                break;
            };

            drop(dependencies);

            cnt += self.try_remove(child)?;

            // TODO proper ops
            // cnt += self.remove(child)?;
        }

        // when_remove
        // SAFETY: we have checked the mutability
        let tid = self.indices.get(&handle).unwrap().tid;
        let storage = self.storages.get(&tid).unwrap().as_ref() as *const _;
        let storage = storage as *mut dyn StorageGeneral;
        unsafe { (*storage).when_remove(self, handle) };

        self.scope.replace(trace.clone());

        // write immediate record
        let mut removed = self.removed.borrow_mut();
        removed.insert(handle.untyped());
        drop(removed);

        // swap_remove shifts the storage indices the cache points to
        self.cache.borrow_mut().remove(&tid);

        self.queue(move |world| {
            // update typetable
            world.indices.remove(&handle.untyped());

            // pop out storage
            let storage = world.storages.get_mut(&tid).unwrap();
            storage.remove(handle.untyped());

            // release the slot, invalidating any outstanding handle to it
            world.deallocate(handle.0);
        });

        Ok(cnt)
    }

    /// Cell-mode removal cannot access the element immediately so we can't return the owned value of removed element.
    pub fn try_remove_with<T: Element>(
        &self,
        handle: Handle<T>,
        recycle: impl FnOnce(T, &World) + Send + 'static,
    ) -> Result<usize, WorldError> {
        let handle = handle.untyped();
        let info = self.info(handle);

        self.available_mut(handle)?;
        let mut cnt = 1;

        let trace = self.scope.borrow().clone();
        self.scope(format!("{} ~{:?}", trace.label, info));

        // cleanup parents' dependencies
        let mut dependencies = self.dependencies.borrow_mut();
        if let Some(deps) = dependencies.0.get_mut(&handle.untyped()) {
            for parent in std::mem::take(&mut deps.parents) {
                let parent_deps = dependencies.0.get_mut(&parent).unwrap();
                parent_deps
                    .children
                    .retain(|child| *child != handle.untyped());
            }
        }
        drop(dependencies);

        // remove children
        loop {
            let mut dependencies = self.dependencies.borrow_mut();
            let Some(dep) = dependencies.0.get(&handle.untyped()) else {
                break;
            };

            let Some(&child) = dep.children.last() else {
                dependencies.0.remove(&handle.untyped());
                break;
            };

            drop(dependencies);

            cnt += self.try_remove(child)?;

            // TODO proper ops
            // cnt += self.remove(child)?;
        }

        // when_remove
        // SAFETY: we have checked the mutability
        let tid = self.indices.get(&handle).unwrap().tid;
        let storage = self.storages.get(&tid).unwrap().as_ref() as *const _;
        let storage = storage as *mut dyn StorageGeneral;
        unsafe { (*storage).when_remove(self, handle) };

        self.scope.replace(trace.clone());

        // write immediate record
        let mut removed = self.removed.borrow_mut();
        removed.insert(handle.untyped());
        drop(removed);

        // swap_remove shifts the storage indices the cache points to
        self.cache.borrow_mut().remove(&tid);

        self.queue(move |world| {
            world.scope(format!("{} ~{{{:?}}}", trace.label, info));

            // update typetable
            world.indices.remove(&handle.untyped());

            // pop out storage
            let storage = world.storages.get_mut(&tid).unwrap();
            let typed_storage = (storage.as_mut() as &mut dyn Any)
                .downcast_mut::<Storage<T>>()
                .unwrap();
            let element = typed_storage.0.swap_remove(&handle).unwrap();

            // perform recycle
            recycle(element, world);

            // release the slot, invalidating any outstanding handle to it
            world.deallocate(handle.0);
        });

        Ok(cnt)
    }

    /// Remove all elements.
    pub fn clear(&self) -> usize {
        let mut cnt = 0;
        for &handle in self.indices.keys() {
            if self.validate(handle).is_ok() {
                cnt += self.remove(handle);
            }
        }

        cnt
    }

    // cache //

    pub fn queue_cache<T: Element>(&self) -> bool {
        let tid = TypeId::of::<T>();
        self.queue(|world| {
            world.cache::<T>();
        });
        !self.cache.borrow().contains_key(&tid)
    }

    // return if a new cache is established
    pub fn cache<T: Element>(&self) -> bool {
        let tid = TypeId::of::<T>();

        let mut cache = self.cache.borrow_mut();
        if cache.contains_key(&tid) {
            return false;
        }

        let Some(storage) = self.storages.get(&tid) else {
            return false;
        };

        let storage = (storage.as_ref() as &dyn Any)
            .downcast_ref::<Storage<T>>()
            .unwrap();

        let mut cached = SmallVec::new();
        for (i, &handle) in storage.0.keys().enumerate() {
            if self.validate(handle).is_err() {
                continue;
            }

            cached.push(i);
        }

        cache.insert(tid, cached);

        return true;
    }

    // commands //

    pub fn commander(&self) -> Commander {
        Commander {
            inner: self.commander.clone(),
            label: self.label().into(),
        }
    }

    pub fn scope(&self, label: impl Into<Cow<'static, str>>) {
        let mut scope = self.scope.borrow_mut();
        *scope = WorldScope {
            label: label.into(),
            autodeps: None,
        };
    }

    pub fn set_autodeps(&self, dep: Option<impl HandleGeneric>) {
        let mut scope = self.scope.borrow_mut();
        scope.autodeps = dep.map(|x| x.untyped());
    }

    pub fn queue(&self, f: impl FnOnce(&mut World) + Send + 'static) {
        let result = self.commander.send(WorldCommand {
            action: Box::new(f),
            label: self.label().into(),
        });
        if let Err(err) = result {
            log::error!("error in world queue ops: {err}");
        }
    }

    pub fn flush(&mut self) {
        let trace = self.scope.get_mut().clone();
        let buf = self.queue.try_iter().collect::<Vec<_>>();
        for cmd in buf {
            self.scope(format!("<{}>", cmd.label));
            (cmd.action)(self);
            self.flush();
        }
        let scope = self.scope.get_mut();
        *scope = trace;
    }

    fn get_storage<T: Element>(&self, tid: TypeId) -> Option<&Storage<T>> {
        let Some(storage) = self.storages.get(&tid) else {
            return None;
        };

        let storage = (storage.as_ref() as &dyn Any)
            .downcast_ref::<Storage<T>>()
            .unwrap();

        Some(storage)
    }

    // validation //

    /// Check whether target element exists, insertion without `flush` will *NOT* be included.
    ///
    /// Validated handles are:
    /// - Not an element just insert
    /// - Not pointing to removed/invalid element
    ///
    /// Validated handles are possible to be:
    /// - With wrong type signature
    /// - Unsafe to Immutably/mutably borrow
    pub fn validate(&self, handle: impl HandleGeneric) -> Result<(), WorldError> {
        let handle = handle.untyped();

        if !self.indices.contains_key(&handle) {
            if self.inserted.borrow().contains(&handle) {
                return Err(WorldError::JustInserted(self.info(handle)));
            }

            return Err(WorldError::InvalidHandle(self.info(handle)));
        }

        if self.removed.borrow().contains(&handle) {
            return Err(WorldError::JustRemoved(self.info(handle)));
        }

        Ok(())
    }

    /// Check whether target element can be borrowed immutably, insertion without
    /// `flush` will *NOT* be included.
    pub fn available(&self, handle: impl HandleGeneric) -> Result<(), WorldError> {
        self.validate(handle)?;
        self.available_unchecked(handle)
    }

    /// Check whether target element can be borrowed immutably, insertion without
    /// `flush` will *NOT* be included.
    fn available_unchecked(&self, handle: impl HandleGeneric) -> Result<(), WorldError> {
        let occupied = self.occupied.borrow();
        if occupied.get(&handle.untyped()).is_some_and(|cnt| *cnt < 0) {
            panic!("{}", WorldError::Unavailable(self.info(handle)));
        }

        Ok(())
    }

    /// Check whether target element can be borrowed mutably, insertion without
    /// `flush` will *NOT* be included.
    pub fn available_mut(&self, handle: impl HandleGeneric) -> Result<(), WorldError> {
        self.validate(handle)?;
        self.available_mut_unchecked(handle)
    }

    /// Check whether target element can be borrowed mutably, insertion without
    /// `flush` will *NOT* be included.
    fn available_mut_unchecked(&self, handle: impl HandleGeneric) -> Result<(), WorldError> {
        let occupied = self.occupied.borrow();
        if occupied.get(&handle.untyped()).is_some_and(|cnt| *cnt != 0) {
            panic!("{}", WorldError::UnavailableMut(self.info(handle)));
        }

        Ok(())
    }

    // fetch //

    pub fn fetch<T: Element>(&self, handle: Handle<T>) -> Ref<'_, T> {
        self.unwrap(self.try_fetch(handle))
    }

    pub fn try_fetch<T: Element>(&self, handle: Handle<T>) -> Result<Ref<'_, T>, WorldError> {
        self.validate(handle)?;
        self.try_fetch_unchecked(handle)
    }

    fn try_fetch_unchecked<T: Element>(&self, handle: Handle<T>) -> Result<Ref<'_, T>, WorldError> {
        self.available_unchecked(handle)?;

        let mut occupied = self.occupied.borrow_mut();
        *occupied.entry(handle.untyped()).or_default() += 1;

        let storage = (self.storages)
            .get(&TypeId::of::<T>())
            .ok_or(WorldError::UnmatchedType(self.info(handle)))?;
        let storage = (storage.as_ref() as &dyn Any)
            .downcast_ref::<Storage<T>>()
            .unwrap();
        let element = (storage.0)
            .get(&handle.untyped())
            .ok_or(WorldError::InvalidHandle(self.info(handle)))? as *const _;

        Ok(Ref {
            ptr: element,
            world: self,
            handle,
        })
    }

    pub fn fetch_mut<T: Element>(&self, handle: Handle<T>) -> RefMut<'_, T> {
        self.unwrap(self.try_fetch_mut(handle))
    }

    pub fn try_fetch_mut<T: Element>(
        &self,
        handle: Handle<T>,
    ) -> Result<RefMut<'_, T>, WorldError> {
        self.validate(handle)?;
        self.try_fetch_mut_unchecked(handle)
    }

    fn try_fetch_mut_unchecked<T: Element>(
        &self,
        handle: Handle<T>,
    ) -> Result<RefMut<'_, T>, WorldError> {
        self.available_mut_unchecked(handle)?;

        let mut occupied = self.occupied.borrow_mut();
        *occupied.entry(handle.untyped()).or_default() -= 1;

        let storage = (self.storages)
            .get(&TypeId::of::<T>())
            .ok_or(WorldError::UnmatchedType(self.info(handle)))?;
        let storage = (storage.as_ref() as &dyn Any)
            .downcast_ref::<Storage<T>>()
            .unwrap();
        let element = (storage.0)
            .get(&handle.untyped())
            .ok_or(WorldError::InvalidHandle(self.info(handle)))? as *const _;
        let element = element as *mut T;

        Ok(RefMut {
            ptr: element,
            world: self,
            handle,
        })
    }

    // singleton //

    pub fn single<T: Element>(&self) -> Handle<T> {
        self.unwrap(self.try_single())
    }

    pub fn single_fetch<T: Element>(&self) -> Ref<'_, T> {
        self.unwrap(self.try_single_fetch())
    }

    pub fn single_fetch_mut<T: Element>(&self) -> RefMut<'_, T> {
        self.unwrap(self.try_single_fetch_mut())
    }

    pub fn single_remove<T: Element>(&self) -> usize {
        self.unwrap(self.try_single_remove::<T>())
    }

    pub fn try_single<T: Element>(&self) -> Result<Handle<T>, WorldError> {
        let tid = TypeId::of::<T>();

        let Some(storage) = self.get_storage::<T>(tid) else {
            return Err(WorldError::SingletonNoSuch(type_name::<T>()));
        };

        let mut ret = None;
        let mut cnt = 0;
        let mut corrupted = 0;
        let cache = self.cache.borrow();
        if let Some(cached) = cache.get(&tid) {
            for &cache in cached {
                let Some((&handle, _)) = storage.0.get_index(cache) else {
                    log::warn!("cache failed");
                    continue;
                };

                cnt += 1;
                ret.replace(handle);
            }
        } else {
            for &handle in storage.0.keys() {
                match self.validate(handle) {
                    Ok(_) => {
                        cnt += 1;
                        ret.replace(handle);
                    }
                    Err(WorldError::JustRemoved(_) | WorldError::JustInserted(_)) => {
                        corrupted += 1;
                    }
                    Err(_) => continue,
                }
            }
        }

        if corrupted != 0 {
            return Err(WorldError::SingletonCorrupted(type_name::<T>()));
        }

        let Some(ret) = ret else {
            return Err(WorldError::SingletonNoSuch(type_name::<T>()));
        };

        if cnt > 1 {
            return Err(WorldError::SingletonTooMany(type_name::<T>(), cnt));
        }

        Ok(ret.cast())
    }

    pub fn try_single_fetch<T: Element>(&self) -> Result<Ref<'_, T>, WorldError> {
        self.try_fetch_unchecked(self.try_single::<T>()?)
    }

    pub fn try_single_fetch_mut<T: Element>(&self) -> Result<RefMut<'_, T>, WorldError> {
        self.try_fetch_mut_unchecked(self.try_single::<T>()?)
    }

    pub fn try_single_remove<T: Element>(&self) -> Result<usize, WorldError> {
        self.try_remove(self.try_single::<T>()?)
    }

    // iteration //

    pub fn foreach<T: Element>(&self, mut f: impl FnMut(Handle<T>)) {
        let tid = TypeId::of::<T>();

        let Some(storage) = self.get_storage::<T>(tid) else {
            return;
        };

        // cache hit optimization
        let cache = self.cache.borrow();
        if let Some(cached) = cache.get(&tid) {
            for &cache in cached {
                let Some((handle, _)) = storage.0.get_index(cache) else {
                    log::warn!("cache failed");
                    continue;
                };

                f(handle.cast());
            }
        } else {
            for &handle in storage.0.keys() {
                if self.validate(handle).is_err() {
                    continue;
                }

                f(handle.cast());
            }
        }
    }

    pub fn foreach_fetch<T: Element>(&self, mut f: impl FnMut(Ref<T>)) {
        self.foreach::<T>(|handle| f(self.try_fetch_unchecked(handle).unwrap()))
    }

    pub fn foreach_fetch_mut<T: Element>(&self, mut f: impl FnMut(RefMut<T>)) {
        self.foreach::<T>(|handle| f(self.try_fetch_mut_unchecked(handle).unwrap()))
    }

    // observer & trigger //

    pub fn observer<E: Send + 'static>(
        &self,
        target: impl HandleGeneric,
        action: impl FnMut(&E, &World) + Send + 'static,
    ) -> HandleAny {
        self.insert(Observer {
            action: Box::new(action),
            label: self.label(),
            target: target.untyped(),
        })
        .untyped()
    }

    /// Will immediately triggered and acquire mutable access to `target`.
    pub fn trigger<E: Send + 'static>(&self, target: impl HandleGeneric, event: &E) -> usize {
        let info = self.info(target);
        let trace = self.scope.borrow().clone();

        if let Err(e) = self.validate(target) {
            // TODO silent return
            log::warn!(
                "{e} [{} .<{}>{:?}]",
                trace.label,
                shorter_type(type_name::<E>()),
                info,
            );
            return 0;
        }

        let mut cnt = 0;
        if let Ok(observers) = self.try_single_fetch::<Observers<E>>()
            && let Some(observers) = observers.members.get(&target.untyped())
        {
            for mut observer in observers.iter().filter_map(|x| self.try_fetch_mut(*x).ok()) {
                self.scope(format!(
                    "{} .[{}]<{}>{:?}",
                    trace.label,
                    observer.label,
                    shorter_type(type_name::<E>()),
                    info,
                ));
                (observer.action)(event, self);
                cnt += 1;
            }
        }

        self.scope.replace(trace);
        cnt
    }

    pub fn queue_trigger<E: Send + 'static>(&self, target: impl HandleGeneric, event: E) {
        let label = self.label();
        self.queue(move |world| {
            world.scope(format!("{} |", label));
            world.trigger(target, &event);
        });
    }

    // dependency //

    /// Declare a dependency relationship. When the `other` Element is removed, this element
    /// will be removed as well. Useful for keeping handle valid.
    pub fn dependency(&self, child: impl HandleGeneric, parent: impl HandleGeneric) {
        if let Err(e) = self.validate(parent)
            && !matches!(e, WorldError::JustInserted(_))
        {
            let err = WorldError::ToxicDependency(self.info(child), self.info(parent));
            log::error!("failed to attach dependency: {err:?} [{}]", self.label());
            return;
        }

        let child = child.untyped();
        let parent = parent.untyped();

        let mut dependencies = self.dependencies.borrow_mut();
        let parent_deps = dependencies.0.entry(parent).or_default();
        parent_deps.children.push(child);
        let child_deps = dependencies.0.entry(child).or_default();
        child_deps.parents.push(parent);
    }

    pub fn info(&self, value: impl HandleGeneric) -> HandleInfo {
        let index = self.indices.get(&value.untyped());
        HandleInfo {
            handle: value.untyped(),
            class: index
                .and_then(|x| self.storages.get(&x.tid))
                .map(|x| x.name())
                .unwrap_or("invalid"),
            trace: index.map(|x| x.trace.clone()).unwrap_or("invalid".into()),
        }
    }

    pub fn label(&self) -> Cow<'static, str> {
        let scope = self.scope.borrow();
        scope.label.clone()
    }

    pub fn stat_deps(&self) -> String {
        let dependencies = self.dependencies.borrow_mut();
        let mut result = String::new();
        let mut cnt = 0;

        result += "==== dependencies statistics ====\n";
        for &handle in self.indices.keys() {
            let Some(deps) = dependencies.0.get(&handle) else {
                result.push_str(&format!(
                    "- orphan node {:?} (no children) \n",
                    self.info(handle)
                ));
                cnt += 1;
                continue;
            };

            if deps.parents.is_empty() {
                result.push_str(&format!("- orphan node {:?}\n", self.info(handle)));
                cnt += 1;
                continue;
            }
        }

        if cnt == 0 {
            result.push_str("everything is fine\n");
        }

        return result;
    }

    fn unwrap<R>(&self, val: Result<R, WorldError>) -> R {
        match val {
            Ok(ret) => return ret,
            Err(err) => {
                panic!("world panic: {} [{}]", err, self.label())
            }
        }
    }
}

impl Default for World {
    fn default() -> Self {
        World::new()
    }
}

/// A world's immutable element reference.
pub struct Ref<'world, T: Element> {
    ptr: *const T,
    world: &'world World,
    handle: Handle<T>,
}

/// A world's limitedly mutable element reference.
pub struct RefMut<'world, T: Element> {
    ptr: *mut T,
    world: &'world World,
    handle: Handle<T>,
}

impl<T: Element> Deref for Ref<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // SAFETY: guaranteed by World's cell_occupied
        unsafe { self.ptr.as_ref().unwrap() }
    }
}

impl<T: Element> Deref for RefMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // SAFETY: guaranteed by World's cell_occupied
        unsafe { self.ptr.as_ref().unwrap() }
    }
}

impl<T: Element> DerefMut for RefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: guaranteed by World's cell_occupied
        unsafe { self.ptr.as_mut().unwrap() }
    }
}

impl<T: Element> Drop for Ref<'_, T> {
    fn drop(&mut self) {
        let mut occupied = self.world.occupied.borrow_mut();
        let cnt = occupied.get_mut(&self.handle.untyped()).unwrap();
        *cnt -= 1;
    }
}

impl<T: Element> Drop for RefMut<'_, T> {
    fn drop(&mut self) {
        let mut occupied = self.world.occupied.borrow_mut();
        let cnt = occupied.get_mut(&self.handle.untyped()).unwrap();
        *cnt += 1;
    }
}

impl<T: Element> Ref<'_, T> {
    pub fn handle(&self) -> Handle<T> {
        self.handle
    }

    pub fn retrieve(self) -> Handle<T> {
        self.handle
    }
}

impl<T: Element> RefMut<'_, T> {
    pub fn handle(&self) -> Handle<T> {
        self.handle
    }

    pub fn retrieve(self) -> Handle<T> {
        self.handle
    }
}

impl Element for () {}

// Commander //

struct WorldCommand {
    action: Box<dyn FnOnce(&mut World) + Send>,
    label: Cow<'static, str>,
}

/// A flexible command access to world.
#[derive(Debug, Clone)]
pub struct Commander {
    inner: Sender<WorldCommand>,
    label: Cow<'static, str>,
}

impl Commander {
    pub fn queue(&self, f: impl FnOnce(&mut World) + Send + 'static) {
        let result = self.inner.send(WorldCommand {
            action: Box::new(f),
            label: self.label.clone(),
        });
        if let Err(err) = result {
            log::error!("error in world queue ops: {err}");
        }
    }
}

// Observer & Trigger //

#[derive(Default)]
struct Observers<E: Send + 'static> {
    members: HashMap<HandleAny, SmallVec<[Handle<Observer<E>>; 1]>>,
}

#[expect(clippy::type_complexity)]
struct Observer<E: Send> {
    action: Box<dyn FnMut(&E, &World) + Send>,
    label: Cow<'static, str>,
    target: HandleAny,
}

impl<E: Send + 'static> Element for Observers<E> {}

impl<E: Send + 'static> Element for Observer<E> {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        match world.try_single_fetch_mut::<Observers<E>>() {
            Ok(mut observers) => {
                let observers = observers.members.entry(self.target).or_default();
                observers.push(this);
            }
            Err(WorldError::SingletonNoSuch(_)) => {
                let mut observers = Observers::<E> {
                    members: HashMap::new(),
                };

                log::trace!("register events: {}", type_name::<E>());

                let observer = observers.members.entry(self.target).or_default();
                observer.push(this);
                world.insert(observers);
            }
            Err(err) => {
                todo!("{err}")
            }
        }

        world.dependency(this, self.target);
    }
}

// Dependency //

#[derive(Default)]
struct Dependencies(HashMap<HandleAny, Dependency>);

#[derive(Default, Clone)]
struct Dependency {
    parents: SmallVec<[HandleAny; 1]>,
    children: SmallVec<[HandleAny; 4]>,
}
