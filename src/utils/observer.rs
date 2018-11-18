use std::sync::{Weak, Arc};

pub trait Listener<E: Clone>: Send + Sync {
    fn on_event(&self, event: &E);
}

impl<E: Clone, F: Fn(&E)> Listener<E> for F
    where F: Send + Sync {
    fn on_event(&self, event: &E) {
        self(event);
    }
}

pub type ListenerHandle = usize;

pub struct Notifier<'l, E> {
    listeners: Vec<(ListenerHandle, Box<Listener<E> + 'l>)>,
    next_handle: ListenerHandle
}

impl<'l, E: Clone> Notifier<'l, E> {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
            next_handle: 0,
        }
    }

    pub fn register<T: Listener<E> + 'l>(&mut self, listener: T) -> ListenerHandle {
        let handle = self.next_handle;
        self.listeners.push((handle, Box::new(listener)));
        self.next_handle += 1;
        handle
    }

    pub fn deregister(&mut self, handle: ListenerHandle) {
        for (i, (stored_handle, _)) in self.listeners.iter().enumerate() {
            if handle == *stored_handle {
                self.listeners.remove(i);
                return;
            }
        }
    }

    pub fn notify(&self, event: E) {
        for (_, listener) in &self.listeners {
            listener.on_event(&event);
        }
    }
}

pub fn weak_listener<T, E, C>(weak_ref: Weak<T>, closure: C) -> impl Listener<E>
    where C: Fn(Arc<T>, &E) + Send + Sync, E: Clone, T: Send + Sync {
    move |event: &E| {
        if let Some(arc) = weak_ref.upgrade() {
            closure(arc, event);
        }
    }
}

pub trait PassThroughListener<E: Clone>: Send + Sync {
    fn on_event(&self, event: E);
}

impl<E: Clone, F: Fn(E)> PassThroughListener<E> for F
    where F: Send + Sync {
    fn on_event(&self, event: E) {
        self(event);
    }
}

pub struct PassThroughNotifier<'l, E> {
    listener: Option<Box<PassThroughListener<E> + 'l>>,
}

impl<'l, E: Clone> PassThroughNotifier<'l, E> {
    pub fn new() -> Self {
        Self {
            listener: None,
        }
    }

    pub fn register<T: PassThroughListener<E> + 'l>(&mut self, listener: T) {
        self.listener = Some(Box::new(listener));
    }

    pub fn deregister(&mut self) {
        self.listener = None;
    }

    pub fn notify(&self, event: E) {
        if let Some(ref listener) = self.listener {
            listener.on_event(event);
        }
    }
}