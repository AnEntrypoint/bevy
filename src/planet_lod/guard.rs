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

#[cfg(test)]
pub fn builds_on_main_thread() -> usize {
    BUILDS.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_are_counted_only_on_the_marked_thread() {
        mark_main_thread();
        note_build();
        note_build();
        let elsewhere = std::thread::spawn(|| {
            note_build();
            builds_on_main_thread()
        })
        .join()
        .unwrap();
        assert_eq!(builds_on_main_thread(), 2);
        assert_eq!(elsewhere, 0);
    }
}
