use crate::queue::FnOnceQueue;
use crate::{task::Task, Stakker};
use std::marker::PhantomData;
use std::mem;
use std::sync::Once;
use std::thread::ThreadId;

// ONCE protects initialisation of LOCKED_TO_THREAD
static ONCE: Once = Once::new();

// Thread onto which this QUEUE is locked.  Once initialised it
// remains constant.
static mut LOCKED_TO_THREAD: Option<ThreadId> = None;

// QUEUE is only ever accessed by the thread registered in
// LOCKED_TO_THREAD, which is guaranteed by the checks below.
static mut QUEUE: FnOnceQueue<Stakker> = FnOnceQueue::new();

// TASK is only ever accessed by the thread registered in
// LOCKED_TO_THREAD, which is guaranteed by the checks below.
static mut TASK: Option<Task> = None;

// Use *const to make it !Send and !Sync
#[derive(Clone)]
pub struct DeferrerAux(PhantomData<*const u8>);

impl DeferrerAux {
    pub(crate) fn new() -> Self {
        // Safety: Once LOCKED_TO_THREAD is set to a ThreadId, QUEUE
        // and TASK will only be accessed from that thread.  This is
        // guaranteed by this check and the fact that DeferrerAux is
        // !Send and !Sync.  So there are no data races on QUEUE or
        // TASK.
        let tid = std::thread::current().id();
        unsafe {
            ONCE.call_once(|| {
                LOCKED_TO_THREAD = Some(tid);
            });
            assert_eq!(
                LOCKED_TO_THREAD,
                Some(tid),
                "Attempted to create another Stakker instance on a different thread.  {}",
                "Enable crate feature `multi-thread` to allow this."
            );
        }
        Self(PhantomData)
    }

    pub(crate) fn swap_queue(&self, queue: &mut FnOnceQueue<Stakker>) {
        // Safety: The running thread has exclusive access to QUEUE;
        // see above.  The operation doesn't call into any method
        // which might also attempt to access the same global.
        unsafe {
            mem::swap(queue, &mut QUEUE);
        }
    }

    #[inline]
    pub fn defer(&self, f: impl FnOnce(&mut Stakker) + 'static) {
        // Safety: The running thread has exclusive access to QUEUE;
        // see above.  The operation doesn't call into any method
        // which might also attempt to access the same global.
        unsafe {
            QUEUE.push(f);
        };
    }

    #[inline]
    pub fn task_replace(&self, task: Option<Task>) -> Option<Task> {
        // Safety: The running thread has exclusive access to TASK;
        // see above.  The operation doesn't call into any method
        // which might also attempt to access the same global.
        unsafe { mem::replace(&mut TASK, task) }
    }
}
