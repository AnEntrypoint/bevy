#[cfg(not(target_arch = "wasm32"))]
pub async fn yield_slice() {
    bevy::tasks::futures_lite::future::yield_now().await;
}

#[cfg(target_arch = "wasm32")]
pub async fn yield_slice() {
    browser::next_task().await;
}

#[cfg(target_arch = "wasm32")]
mod browser {
    use std::cell::RefCell;
    use std::task::{Poll, Waker};
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::{JsCast, JsValue};
    use web_sys::{MessageChannel, MessagePort};

    struct Channel {
        post: MessagePort,
        _receive: MessagePort,
        _listener: Closure<dyn FnMut(JsValue)>,
        posted: u64,
        delivered: u64,
        waiting: Vec<Waker>,
    }

    thread_local! {
        static CHANNEL: RefCell<Option<Channel>> = const { RefCell::new(None) };
    }

    impl Channel {
        fn open() -> Self {
            let channel = MessageChannel::new().expect("a message channel");
            let listener = Closure::<dyn FnMut(JsValue)>::new(|_: JsValue| {
                let waiting = CHANNEL.with_borrow_mut(|slot| {
                    slot.as_mut().map_or_else(Vec::new, |channel| {
                        channel.delivered += 1;
                        std::mem::take(&mut channel.waiting)
                    })
                });
                waiting.into_iter().for_each(Waker::wake);
            });
            let receive = channel.port2();
            receive.set_onmessage(Some(listener.as_ref().unchecked_ref()));
            Self { post: channel.port1(), _receive: receive, _listener: listener, posted: 0, delivered: 0, waiting: Vec::new() }
        }
    }

    fn with_channel<T>(use_channel: impl FnOnce(&mut Channel) -> T) -> T {
        CHANNEL.with_borrow_mut(|slot| use_channel(slot.get_or_insert_with(Channel::open)))
    }

    pub async fn next_task() {
        let ticket = with_channel(|channel| {
            channel.posted += 1;
            channel.post.post_message(&JsValue::NULL).expect("a posted message");
            channel.posted
        });
        std::future::poll_fn(move |cx| {
            with_channel(|channel| {
                if channel.delivered >= ticket {
                    Poll::Ready(())
                } else {
                    channel.waiting.push(cx.waker().clone());
                    Poll::Pending
                }
            })
        })
        .await
    }
}
