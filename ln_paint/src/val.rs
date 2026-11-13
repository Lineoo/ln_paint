use ln_world::{Element, Handle, HandleGeneric, World};

type TransFunc<T> = Box<dyn Fn(&World) -> T + Send + 'static>;

pub struct Val<T> {
    pub val: T,
}

/// maybe change, maybe not
pub struct ValChanged;

/// Binding a function for data, and listen to nodes.
pub struct ValBind<'w, T: Send + 'static> {
    world: &'w World,
    handle: Handle<Val<T>>,
    fun: Handle<Val<TransFunc<T>>>,
}

impl<T> Val<T> {
    pub const fn new(val: T) -> Self {
        Val { val }
    }
}

impl<T: Send + 'static> ValBind<'_, T> {
    pub fn new(world: &World, fun: impl Fn(&World) -> T + Send + 'static) -> ValBind<'_, T> {
        let fun = world.insert(Val::new(Box::new(fun) as TransFunc<T>));
        let handle = world.insert_with(move |world| {
            let fun = world.fetch(fun);
            Val::new((fun.val)(world))
        });
        ValBind { world, handle, fun }
    }

    pub fn attach(
        world: &World,
        handle: Handle<Val<T>>,
        fun: impl Fn(&World) -> T + Send + 'static,
    ) -> ValBind<'_, T> {
        let fun = world.insert(Val::new(Box::new(fun) as TransFunc<T>));
        world.queue(move |world| {
            let fun = world.fetch(fun);
            let mut that = world.fetch_mut(handle);
            that.val = (fun.val)(world);
            world.queue_trigger(that.handle(), ValChanged);
        });
        ValBind { world, handle, fun }
    }

    pub fn transfer<U: Send + 'static>(
        world: &World,
        src: Handle<Val<U>>,
        fun: impl Fn(&U) -> T + Send + 'static,
    ) -> Handle<Val<T>> {
        ValBind::new(world, move |world| fun(&world.fetch(src).val))
            .listen(src)
            .finish()
    }

    /// standard val bind observer
    pub fn listen(self, this: impl HandleGeneric) -> Self {
        let ValBind { world, handle, fun } = self;
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
        let ValBind { world, handle, .. } = self;
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

impl<T: Send + 'static> Element for Val<T> {}
