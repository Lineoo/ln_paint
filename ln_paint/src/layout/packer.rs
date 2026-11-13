use ln_world::{Handle, HandleGeneric, World};

use crate::layout::val::{Val, ValChanged};

type F<T> = dyn Fn(&World) -> T + Send + 'static;

/// Binding a function for data, and listen to nodes.
pub struct Packer<'w, T: Send + 'static> {
    world: &'w World,
    handle: Handle<Val<T>>,
    fun: Handle<Val<Box<F<T>>>>,
}

impl<T: Send + 'static> Packer<'_, T> {
    pub fn new(world: &World, fun: impl Fn(&World) -> T + Send + 'static) -> Packer<'_, T> {
        let fun = world.insert(Val::new(Box::new(fun) as Box<F<T>>));
        let handle = world.insert_with(move |world| {
            let fun = world.fetch(fun);
            Val::new((fun.val)(world))
        });
        Packer { world, handle, fun }
    }

    pub fn attach(
        world: &World,
        handle: Handle<Val<T>>,
        fun: impl Fn(&World) -> T + Send + 'static,
    ) -> Packer<'_, T> {
        let fun = world.insert(Val::new(Box::new(fun) as Box<F<T>>));
        world.queue(move |world| {
            let fun = world.fetch(fun);
            let mut that = world.fetch_mut(handle);
            that.val = (fun.val)(world);
            world.queue_trigger(that.handle(), ValChanged);
        });
        Packer { world, handle, fun }
    }

    pub fn transfer<U: Send + 'static>(
        world: &World,
        src: Handle<Val<U>>,
        fun: impl Fn(&U) -> T + Send + 'static,
    ) -> Handle<Val<T>> {
        Packer::new(world, move |world| fun(&world.fetch(src).val))
            .listen(src)
            .finish()
    }

    /// standard val bind observer
    pub fn listen(self, this: impl HandleGeneric) -> Self {
        let Packer { world, handle, fun } = self;
        world.observer(this, move |ValChanged, world| {
            let fun = world.fetch(fun);
            let mut that = world.fetch_mut(handle);
            that.val = (fun.val)(world);
            world.queue_trigger(that.handle(), ValChanged);
        });
        self
    }

    /// any observer
    pub fn observer<E: Send + 'static>(
        self,
        this: impl HandleGeneric,
        mut action: impl FnMut(&E, &World) -> T + Send + 'static,
    ) -> Self {
        let Packer { world, handle, .. } = self;
        world.observer(this, move |event: &E, world| {
            let mut that = world.fetch_mut(handle);
            that.val = (action)(event, world);
            world.queue_trigger(that.handle(), ValChanged);
        });
        self
    }

    pub fn finish(self) -> Handle<Val<T>> {
        self.handle
    }
}
