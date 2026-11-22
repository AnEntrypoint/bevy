use std::cell::Cell;

thread_local! {
    static MARKED_MAIN: Cell<bool> = const { Cell::new(false) };
    static BUILDS: Cell<usize> = const { Cell::new(0) };
}

pub fn mark_main_thread() {
    MARKED_MAIN.set(true);
}

pub fn note_build() {
    if MARKED_MAIN.get() {
        BUILDS.set(BUILDS.get() + 1);
    }
}

pub fn builds_on_main_thread() -> usize {
    BUILDS.get()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_lod::collider::{Chart, Lattice, build_collider};
    use bevy::prelude::Transform;

    fn on_own_thread<T: Send + 'static>(job: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::spawn(job).join().unwrap()
    }

    #[test]
    fn builds_are_counted_only_on_the_marked_thread() {
        let (marked, elsewhere) = on_own_thread(|| {
            mark_main_thread();
            note_build();
            note_build();
            let elsewhere = on_own_thread(|| {
                note_build();
                builds_on_main_thread()
            });
            (builds_on_main_thread(), elsewhere)
        });
        assert_eq!((marked, elsewhere), (2, 0));
    }

    #[test]
    fn a_collider_build_on_the_marked_thread_is_counted() {
        let counted = on_own_thread(|| {
            mark_main_thread();
            let chart = Chart { pose: Transform::IDENTITY, body: [0.0, 0.0], height: Box::new(|_, _| 0.0) };
            build_collider(&Lattice::default(), chart);
            builds_on_main_thread()
        });
        assert_eq!(counted, 1);
    }
}
