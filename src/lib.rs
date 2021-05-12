#![no_std]

use core::sync::atomic::{AtomicUsize, Ordering, AtomicPtr};
use core::future::Future;
use core::task::{Context, Poll, Waker};
use core::pin::Pin;
use core::marker::{PhantomData, PhantomPinned};

pub struct WakerList {
    links: WakerListItemLinks,
    _marker: PhantomPinned,
}

impl WakerList {
    pub fn new() -> Self {
        WakerList {
            links: WakerListItemLinks::default(),
            _marker: PhantomPinned,
        }
    }

    pub fn insert(&self, _item: Pin<&mut WakerListItem>) {
        todo!();
    }

    pub fn remove(&self, _item: Pin<&mut WakerListItem>) {
        todo!();
    }

    pub fn wake_all(&self) {
        todo!()
    }
}

#[derive(Default)]
struct WakerListItemLinks {
    prev: AtomicPtr<WakerListItemLinks>,
    next: AtomicPtr<WakerListItemLinks>,
}

impl Drop for WakerListItemLinks {
    fn drop(&mut self) {
        let prev = self.prev.load(Ordering::SeqCst);
        let next = self.next.load(Ordering::SeqCst);
        if !prev.is_null() {
            unsafe {
                (*prev).next.store(next, Ordering::SeqCst);
            }
        }
        if !next.is_null() {
            unsafe {
                (*next).prev.store(prev, Ordering::SeqCst);
            }
        }
    }
}

pub struct WakerListItem {
    links: WakerListItemLinks,
    waker: Waker,
    _marker: PhantomData<*const ()>, // !Send + !Sync
    _marker2: PhantomPinned,
}

impl WakerListItem {
    pub fn new(waker: Waker) -> Self {
        WakerListItem {
            links: WakerListItemLinks::default(),
            waker,
            _marker: PhantomData,
            _marker2: PhantomPinned,
        }
    }
}

impl Drop for WakerListItem {
    fn drop(&mut self) {
        todo!()
    }
}

pub struct EventSource {
    counter: AtomicUsize,
    waker_list: WakerList,
}

impl EventSource {
    pub fn event(&self) -> EventTracker {
        let start_count = self.counter.load(Ordering::SeqCst);
        EventTracker {
            source: self,
            start_count,
        }
    }

    pub fn signal(&self) {
        self.counter.fetch_add(1, Ordering::SeqCst);
        self.waker_list.wake_all();
    }
}

pub struct EventTracker<'a> {
    source: &'a EventSource,
    start_count: usize,
}

impl<'a> EventTracker<'a> {
    pub async fn wait(self) {
        EventTrackerFuture {
            source: self.source,
            start_count: self.start_count,
            list_item: None,
        }.await
    }
}

pub struct EventTrackerFuture<'a> {
    source: &'a EventSource,
    start_count: usize,
    list_item: Option<WakerListItem>,
}

impl Future for EventTrackerFuture<'_> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let counter = self.source.counter.load(Ordering::SeqCst);
        if counter != self.start_count {
            Poll::Ready(())
        } else {
            let source = self.source;

            let list_item = unsafe { self.map_unchecked_mut(|s| &mut s.list_item) };

            //let list_item = &mut self.get_mut().list_item;
            //let item = list_item.get_or_insert_with(|| WakerListItem::new(cx.waker().clone()));
            let item = unsafe {
                list_item.map_unchecked_mut(|i| i.get_or_insert_with(|| WakerListItem::new(cx.waker().clone())))
            };

            // Replace waker if necessary
            let waker = unsafe { &mut item.get_unchecked_mut().waker };
            if !waker.will_wake(cx.waker()) {
                *waker = cx.waker().clone();
            }

            // Obtain a projection in unsafe way
            //let item = unsafe { Pin::new_unchecked(item) };

            source.waker_list.insert(item);

            Poll::Pending
        }
    }
}
