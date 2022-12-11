#[derive(Clone)]
pub struct Event<'a, A> {
    subscribers: Vec<&'a (dyn Fn(&A) + Sync)>,
}

impl<'a, A> Event<'a, A> {
    pub fn new() -> Self {
        Event {
            subscribers: vec![],
        }
    }

    pub fn call(&self, arg: A) {
        for f in &self.subscribers {
            f(&arg);
        }
    }

    pub fn sub(&mut self, handler: &'a (dyn Fn(&A) + Sync)) {
        self.subscribers.push(handler);
    }
}

impl<'a, A> Default for Event<'a, A> {
    fn default() -> Self {
        Self::new()
    }
}
