use dioxus::prelude::*;

pub(crate) trait Refresh<T> {
    fn refresh(self, fresh: Vec<T>);
}

impl<T: PartialEq + 'static> Refresh<T> for Store<Vec<T>> {
    fn refresh(mut self, fresh: Vec<T>) {
        let kept = self.peek().len();
        if fresh.len() < kept {
            self.truncate(fresh.len());
        }
        let mut fresh = fresh.into_iter();
        for (index, item) in fresh.by_ref().take(kept).enumerate() {
            if *self.index(index).peek() != item {
                self.index(index).set(item);
            }
        }
        for item in fresh {
            self.push(item);
        }
    }
}
