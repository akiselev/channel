extern crate channel;
extern crate crossbeam;

use std::any::Any;
use std::thread;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::time::Duration;

use channel::{bounded, select, unbounded, Receiver, Sender};
use channel::{RecvError, RecvTimeoutError, SendError, SendTimeoutError, TryRecvError, TrySendError};

fn ms(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

#[test]
fn smoke1() {
    let mut iters = 0;
    let (tx1, rx1) = unbounded();
    let (tx2, rx2) = unbounded();

    tx1.send(1).unwrap();
    loop {
        iters += 1;
        if let Ok(v) = select::recv(&rx1) {
            assert_eq!(v, 1);
            break;
        }
        if let Ok(_) = select::recv(&rx2) {
            panic!();
        }
    }

    tx2.send(2).unwrap();
    loop {
        iters += 1;
        if let Ok(_) = select::recv(&rx1) {
            panic!();
        }
        if let Ok(v) = select::recv(&rx2) {
            assert_eq!(v, 2);
            break;
        }
    }

    assert!(iters < 50);
}

#[test]
fn smoke2() {
    let mut iters = 0;
    let (_tx1, rx1) = unbounded::<i32>();
    let (_tx2, rx2) = unbounded::<i32>();
    let (_tx3, rx3) = unbounded::<i32>();
    let (_tx4, rx4) = unbounded::<i32>();
    let (tx5, rx5) = unbounded::<i32>();

    tx5.send(5).unwrap();

    loop {
        iters += 1;
        if let Ok(_) = select::recv(&rx1) {
            panic!();
        }
        if let Ok(_) = select::recv(&rx2) {
            panic!();
        }
        if let Ok(_) = select::recv(&rx3) {
            panic!();
        }
        if let Ok(_) = select::recv(&rx4) {
            panic!();
        }
        if let Ok(x) = select::recv(&rx5) {
            assert_eq!(x, 5);
            break;
        }
    }

    assert!(iters < 50);
}

#[test]
fn disconnected() {
    let mut iters = 0;
    let (tx1, rx1) = unbounded::<i32>();
    let (tx2, rx2) = unbounded::<i32>();

    crossbeam::scope(|s| {
        s.spawn(|| {
            thread::sleep(ms(500));
            drop(tx1);
        });

        loop {
            iters += 1;
            if let Ok(_) = select::recv(&rx1) {
                panic!();
            }
            if let Ok(_) = select::recv(&rx2) {
                panic!();
            }
            if select::disconnected() {
                panic!();
            }
            if select::timeout(ms(1000)) {
                break;
            }
        }
    });

    crossbeam::scope(|s| {
        s.spawn(|| {
            thread::sleep(ms(500));
            drop(tx2);
        });

        loop {
            iters += 1;
            if let Ok(_) = select::recv(&rx1) {
                panic!();
            }
            if let Ok(_) = select::recv(&rx2) {
                panic!();
            }
            if select::disconnected() {
                break;
            }
            if select::timeout(ms(1000)) {
                panic!();
            }
        }
    });

    assert!(iters < 50);
}

#[test]
fn would_block() {
    let mut iters = 0;
    let (tx1, rx1) = unbounded::<i32>();
    let (tx2, rx2) = unbounded::<i32>();

    drop(tx1);
    loop {
        iters += 1;
        if let Ok(_) = select::recv(&rx1) {
            panic!();
        }
        if let Ok(_) = select::recv(&rx2) {
            panic!();
        }
        if select::disconnected() {
            panic!();
        }
        if select::would_block() {
            break;
        }
        if select::timeout(ms(0)) {
            panic!();
        }
    }

    tx2.send(2).unwrap();
    loop {
        iters += 1;
        if let Ok(_) = select::recv(&rx1) {
            panic!();
        }
        if let Ok(x) = select::recv(&rx2) {
            assert_eq!(x, 2);
            break;
        }
        if select::disconnected() {
            panic!();
        }
        if select::would_block() {
            panic!();
        }
        if select::timeout(ms(0)) {
            panic!();
        }
    }

    drop(tx2);
    loop {
        iters += 1;
        if let Ok(_) = select::recv(&rx1) {
            panic!();
        }
        if let Ok(_) = select::recv(&rx2) {
            panic!();
        }
        if select::disconnected() {
            break;
        }
        if select::would_block() {
            panic!();
        }
        if select::timeout(ms(0)) {
            panic!();
        }
    }

    assert!(iters < 50);
}

#[test]
fn timeout() {
    let mut iters = 0;
    let (_tx1, rx1) = unbounded::<i32>();
    let (tx2, rx2) = unbounded::<i32>();

    crossbeam::scope(|s| {
        s.spawn(|| {
            thread::sleep(ms(1500));
            tx2.send(2).unwrap();
        });

        loop {
            iters += 1;
            if let Ok(_) = select::recv(&rx1) {
                panic!();
            }
            if let Ok(_) = select::recv(&rx2) {
                panic!();
            }
            if select::timeout(ms(1000)) {
                break;
            }
        }

        loop {
            iters += 1;
            if let Ok(_) = select::recv(&rx1) {
                panic!();
            }
            if let Ok(x) = select::recv(&rx2) {
                assert_eq!(x, 2);
                break;
            }
            if select::timeout(ms(1000)) {
                panic!();
            }
        }
    });

    assert!(iters < 50);
}

#[test]
fn unblocks() {
    let mut iters = 0;
    let (tx1, rx1) = bounded(0);
    let (tx2, rx2) = bounded(0);

    crossbeam::scope(|s| {
        s.spawn(|| {
            thread::sleep(ms(500));
            tx2.send(2).unwrap();
        });

        loop {
            iters += 1;
            if let Ok(_) = select::recv(&rx1) {
                panic!();
            }
            if let Ok(x) = select::recv(&rx2) {
                assert_eq!(x, 2);
                break;
            }
            if select::timeout(ms(1000)) {
                panic!();
            }
        }
    });

    crossbeam::scope(|s| {
        s.spawn(|| {
            thread::sleep(ms(500));
            assert_eq!(rx1.recv().unwrap(), 1);
        });

        loop {
            iters += 1;
            if let Ok(()) = select::send(&tx1, 1) {
                break;
            }
            if let Ok(()) = select::send(&tx2, 2) {
                panic!();
            }
            if select::timeout(ms(1000)) {
                panic!();
            }
        }
    });

    assert!(iters < 50);
}

#[test]
fn both_ready() {
    let mut iters = 0;
    let (tx1, rx1) = bounded(0);
    let (tx2, rx2) = bounded(0);

    crossbeam::scope(|s| {
        s.spawn(|| {
            thread::sleep(ms(500));
            tx1.send(1).unwrap();
            assert_eq!(rx2.recv().unwrap(), 2);
        });

        for _ in 0..2 {
            loop {
                iters += 1;
                if let Ok(x) = select::recv(&rx1) {
                    assert_eq!(x, 1);
                    break;
                }
                if let Ok(()) = select::send(&tx2, 2) {
                    break;
                }
            }
        }
    });

    assert!(iters < 50);
}

#[test]
fn no_starvation() {
    const N: usize = 10;

    let done_rx = &(0..N).map(|_| AtomicBool::new(false)).collect::<Vec<_>>();
    let done_tx = &(0..N).map(|_| AtomicBool::new(false)).collect::<Vec<_>>();

    while !done_rx.iter().all(|x| x.load(SeqCst)) || !done_tx.iter().all(|x| x.load(SeqCst)) {
        crossbeam::scope(|s| {
            let rxs = (0..N)
                .map(|i| {
                    let (tx, rx) = unbounded();
                    tx.send(i).unwrap();
                    rx
                })
                .collect::<Vec<_>>();

            let txs = (0..N)
                .map(|i| {
                    let (tx, rx) = bounded(100);
                    s.spawn(move || if let Ok(x) = rx.recv() {
                        assert_eq!(x, i);
                        done_tx[i].store(true, SeqCst);
                    });
                    tx
                })
                .collect::<Vec<_>>();

            let mut iters = 0;

            'select: loop {
                iters += 1;

                for rx in &rxs {
                    if let Ok(x) = select::recv(&rx) {
                        done_rx[x].store(true, SeqCst);
                        break 'select;
                    }
                }

                for (i, tx) in txs.iter().enumerate() {
                    if let Ok(()) = select::send(&tx, i) {
                        break 'select;
                    }
                }
            }

            assert!(iters < 50);
        });
    }
}

#[test]
fn loop_try() {
    for _ in 0..20 {
        let (tx1, rx1) = bounded::<i32>(0);
        let (tx2, rx2) = bounded::<i32>(0);

        crossbeam::scope(|s| {
            s.spawn(|| loop {
                match tx1.try_send(1) {
                    Ok(()) => break,
                    Err(TrySendError::Disconnected(_)) => break,
                    Err(TrySendError::Full(_)) => continue,
                }
            });

            s.spawn(|| loop {
                match rx2.try_recv() {
                    Ok(x) => {
                        assert_eq!(x, 2);
                        break;
                    }
                    Err(TryRecvError::Disconnected) => break,
                    Err(TryRecvError::Empty) => continue,
                }
            });

            s.spawn(|| {
                let mut iters = 0;
                thread::sleep(ms(500));

                loop {
                    iters += 1;
                    if let Ok(x) = select::recv(&rx1) {
                        assert_eq!(x, 1);
                        break;
                    }
                    if let Ok(_) = select::send(&tx2, 2) {
                        break;
                    }
                    if select::disconnected() {
                        panic!();
                    }
                    if select::timeout(ms(500)) {
                        panic!();
                    }
                }

                drop(rx1);
                drop(tx2);
                assert!(iters < 1000);
            });
        });
    }
}

#[test]
fn cloning1() {
    crossbeam::scope(|s| {
        let mut iters = 0;
        let (tx1, rx1) = unbounded::<i32>();
        let (_tx2, rx2) = unbounded::<i32>();
        let (tx3, rx3) = unbounded::<()>();

        s.spawn(move || {
            rx3.recv().unwrap();
            tx1.clone();
            assert_eq!(rx3.try_recv(), Err(TryRecvError::Empty));
            tx1.send(1).unwrap();
            rx3.recv().unwrap();
        });

        tx3.send(()).unwrap();
        loop {
            iters += 1;
            if let Ok(_) = select::recv(&rx1) {
                break;
            }
            if let Ok(_) = select::recv(&rx2) {
                panic!();
            }
        }

        tx3.send(()).unwrap();
        assert!(iters < 50);
    });
}

#[test]
fn cloning2() {
    crossbeam::scope(|s| {
        let mut iters = 0;
        let (tx1, rx1) = unbounded::<()>();
        let (tx2, rx2) = unbounded::<()>();
        let (_tx3, _rx3) = unbounded::<()>();

        s.spawn(move || loop {
            iters += 1;
            if let Ok(_) = select::recv(&rx1) {
                panic!();
            }
            if let Ok(_) = select::recv(&rx2) {
                break;
            }
        });

        thread::sleep(ms(500));
        drop(tx1.clone());
        tx2.send(()).unwrap();

        assert!(iters < 50);
    })
}

#[test]
fn preflight1() {
    let (tx, rx) = unbounded();
    tx.send(()).unwrap();

    let mut iters = 0;
    loop {
        iters += 1;
        if let Ok(_) = select::recv(&rx) {
            break;
        }
    }
    assert!(iters < 10);
}

#[test]
fn preflight2() {
    let (tx, rx) = unbounded();
    drop(tx.clone());
    tx.send(()).unwrap();
    drop(tx);

    let mut iters = 0;
    loop {
        iters += 1;
        if let Ok(_) = select::recv(&rx) {
            break;
        }
    }
    assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));
    assert!(iters < 10);
}

#[test]
fn preflight3() {
    let (tx, rx) = unbounded();
    drop(tx.clone());
    tx.send(()).unwrap();
    drop(tx);
    rx.recv().unwrap();

    let mut iters = 0;
    loop {
        iters += 1;
        if let Ok(_) = select::recv(&rx) {
            panic!();
        }
        if select::disconnected() {
            break;
        }
    }
    assert!(iters < 10);
}

#[test]
fn stress_recv() {
    let (tx1, rx1) = unbounded();
    let (tx2, rx2) = bounded(5);
    let (tx3, rx3) = bounded(100);

    crossbeam::scope(|s| {
        s.spawn(|| for i in 0..10_000 {
            tx1.send(i).unwrap();
            rx3.recv().unwrap();

            tx2.send(i).unwrap();
            rx3.recv().unwrap();
        });

        for i in 0..10_000 {
            let mut iters = 0;

            for _ in 0..2 {
                loop {
                    iters += 1;
                    if let Ok(x) = select::recv(&rx1) {
                        assert_eq!(x, i);
                        break;
                    }
                    if let Ok(x) = select::recv(&rx2) {
                        assert_eq!(x, i);
                        break;
                    }
                }

                tx3.send(()).unwrap();
            }

            assert!(iters < 50);
        }
    });
}

#[test]
fn stress_send() {
    let (tx1, rx1) = bounded(0);
    let (tx2, rx2) = bounded(0);
    let (tx3, rx3) = bounded(100);

    crossbeam::scope(|s| {
        s.spawn(|| for i in 0..10_000 {
            assert_eq!(rx1.recv().unwrap(), i);
            assert_eq!(rx2.recv().unwrap(), i);
            rx3.recv().unwrap();
        });

        for i in 0..10_000 {
            let mut iters = 0;

            for _ in 0..2 {
                loop {
                    iters += 1;
                    if let Ok(()) = select::send(&tx1, i) {
                        break;
                    }
                    if let Ok(()) = select::send(&tx2, i) {
                        break;
                    }
                }
            }
            tx3.send(()).unwrap();

            assert!(iters < 50);
        }
    });
}

#[test]
fn stress_mixed() {
    let (tx1, rx1) = bounded(0);
    let (tx2, rx2) = bounded(0);
    let (tx3, rx3) = bounded(100);

    crossbeam::scope(|s| {
        s.spawn(|| for i in 0..10_000 {
            tx1.send(i).unwrap();
            assert_eq!(rx2.recv().unwrap(), i);
            rx3.recv().unwrap();
        });

        for i in 0..10_000 {
            let mut iters = 0;

            for _ in 0..2 {
                loop {
                    iters += 1;
                    if let Ok(x) = select::recv(&rx1) {
                        assert_eq!(x, i);
                        break;
                    }
                    if let Ok(()) = select::send(&tx2, i) {
                        break;
                    }
                }
            }
            tx3.send(()).unwrap();

            assert!(iters < 50);
        }
    });
}

#[test]
fn stress_timeout_two_threads() {
    const COUNT: usize = 20;

    let (tx, rx) = bounded(2);

    crossbeam::scope(|s| {
        s.spawn(|| for i in 0..COUNT {
            if i % 2 == 0 {
                thread::sleep(ms(500));
            }
            'outer: loop {
                loop {
                    if let Ok(()) = select::send(&tx, i) {
                            break 'outer;
                        }
                    if select::timeout(ms(100)) {
                        break;
                    }
                }
            }
        });

        s.spawn(|| for i in 0..COUNT {
            if i % 2 == 0 {
                thread::sleep(ms(500));
            }
            'outer: loop {
                loop {
                    if let Ok(x) = select::recv(&rx) {
                        assert_eq!(x, i);
                        break 'outer;
                    }
                    if select::timeout(ms(100)) {
                        break;
                    }
                }
            }
        });
    });
}

struct WrappedSender<T>(Sender<T>);

impl<T> WrappedSender<T> {
    pub fn try_send(&self, mut value: T) -> Result<(), TrySendError<T>> {
        let mut iters = 0;
        loop {
            iters += 1;
            assert!(iters < 20);

            if let Err(v) = select::send(&self.0, value) {
                value = v;
            } else {
                return Ok(());
            }
            if select::disconnected() {
                return Err(TrySendError::Disconnected(value));
            }
            if select::would_block() {
                return Err(TrySendError::Full(value));
            }
        }
    }

    pub fn send(&self, mut value: T) -> Result<(), SendError<T>> {
        let mut iters = 0;
        loop {
            iters += 1;
            assert!(iters < 20);

            if let Err(v) = select::send(&self.0, value) {
                value = v;
            } else {
                return Ok(());
            }
            if select::disconnected() {
                return Err(SendError(value));
            }
        }
    }

    pub fn send_timeout(&self, mut value: T, dur: Duration) -> Result<(), SendTimeoutError<T>> {
        let mut iters = 0;
        loop {
            iters += 1;
            assert!(iters < 20);

            if let Err(v) = select::send(&self.0, value) {
                value = v;
            } else {
                return Ok(());
            }
            if select::disconnected() {
                return Err(SendTimeoutError::Disconnected(value));
            }
            if select::timeout(dur) {
                return Err(SendTimeoutError::Timeout(value));
            }
        }
    }
}

struct WrappedReceiver<T>(Receiver<T>);

impl<T> WrappedReceiver<T> {
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        let mut iters = 0;
        loop {
            iters += 1;
            assert!(iters < 20);

            if let Ok(v) = select::recv(&self.0) {
                return Ok(v);
            }
            if select::disconnected() {
                return Err(TryRecvError::Disconnected);
            }
            if select::would_block() {
                return Err(TryRecvError::Empty);
            }
        }
    }

    pub fn recv(&self) -> Result<T, RecvError> {
        let mut iters = 0;
        loop {
            iters += 1;
            assert!(iters < 20);

            if let Ok(v) = select::recv(&self.0) {
                return Ok(v);
            }
            if select::disconnected() {
                return Err(RecvError);
            }
        }
    }

    pub fn recv_timeout(&self, dur: Duration) -> Result<T, RecvTimeoutError> {
        let mut iters = 0;
        loop {
            iters += 1;
            assert!(iters < 20);

            if let Ok(v) = select::recv(&self.0) {
                return Ok(v);
            }
            if select::disconnected() {
                return Err(RecvTimeoutError::Disconnected);
            }
            if select::timeout(dur) {
                return Err(RecvTimeoutError::Timeout);
            }
        }
    }
}

#[test]
fn recv() {
    let (tx, rx) = bounded(100);
    let tx = WrappedSender(tx);
    let rx = WrappedReceiver(rx);

    crossbeam::scope(|s| {
        s.spawn(move || {
            assert_eq!(rx.recv(), Ok(7));
            thread::sleep(ms(1000));
            assert_eq!(rx.recv(), Ok(8));
            thread::sleep(ms(1000));
            assert_eq!(rx.recv(), Ok(9));
            assert_eq!(rx.recv(), Err(RecvError));
        });
        s.spawn(move || {
            thread::sleep(ms(1500));
            assert_eq!(tx.send(7), Ok(()));
            assert_eq!(tx.send(8), Ok(()));
            assert_eq!(tx.send(9), Ok(()));
        });
    });

    let (tx, rx) = bounded(0);
    let tx = WrappedSender(tx);
    let rx = WrappedReceiver(rx);

    crossbeam::scope(|s| {
        s.spawn(move || {
            assert_eq!(rx.recv(), Ok(7));
            thread::sleep(ms(1000));
            assert_eq!(rx.recv(), Ok(8));
            thread::sleep(ms(1000));
            assert_eq!(rx.recv(), Ok(9));
            assert_eq!(rx.recv(), Err(RecvError));
        });
        s.spawn(move || {
            thread::sleep(ms(1500));
            assert_eq!(tx.send(7), Ok(()));
            assert_eq!(tx.send(8), Ok(()));
            assert_eq!(tx.send(9), Ok(()));
        });
    });
}

#[test]
fn recv_timeout() {
    let (tx, rx) = bounded(100);
    let tx = WrappedSender(tx);
    let rx = WrappedReceiver(rx);

    crossbeam::scope(|s| {
        s.spawn(move || {
            assert_eq!(rx.recv_timeout(ms(1000)), Err(RecvTimeoutError::Timeout));
            assert_eq!(rx.recv_timeout(ms(1000)), Ok(7));
            assert_eq!(
                rx.recv_timeout(ms(1000)),
                Err(RecvTimeoutError::Disconnected)
            );
        });
        s.spawn(move || {
            thread::sleep(ms(1500));
            assert_eq!(tx.send(7), Ok(()));
        });
    });
}

#[test]
fn try_recv() {
    let (tx, rx) = bounded(100);
    let tx = WrappedSender(tx);
    let rx = WrappedReceiver(rx);

    crossbeam::scope(|s| {
        s.spawn(move || {
            assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
            thread::sleep(ms(1500));
            assert_eq!(rx.try_recv(), Ok(7));
            thread::sleep(ms(500));
            assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));
        });
        s.spawn(move || {
            thread::sleep(ms(1000));
            assert_eq!(tx.send(7), Ok(()));
        });
    });

    let (tx, rx) = bounded(0);
    let tx = WrappedSender(tx);
    let rx = WrappedReceiver(rx);

    crossbeam::scope(|s| {
        s.spawn(move || {
            assert_eq!(rx.recv_timeout(ms(1000)), Err(RecvTimeoutError::Timeout));
            assert_eq!(rx.recv_timeout(ms(1000)), Ok(7));
            assert_eq!(
                rx.recv_timeout(ms(1000)),
                Err(RecvTimeoutError::Disconnected)
            );
        });
        s.spawn(move || {
            thread::sleep(ms(1500));
            assert_eq!(tx.send(7), Ok(()));
        });
    });
}

#[test]
fn send() {
    let (tx, rx) = bounded(1);
    let tx = WrappedSender(tx);
    let rx = WrappedReceiver(rx);

    crossbeam::scope(|s| {
        s.spawn(move || {
            assert_eq!(tx.send(7), Ok(()));
            thread::sleep(ms(1000));
            assert_eq!(tx.send(8), Ok(()));
            thread::sleep(ms(1000));
            assert_eq!(tx.send(9), Ok(()));
            thread::sleep(ms(1000));
            assert_eq!(tx.send(10), Err(SendError(10)));
        });
        s.spawn(move || {
            thread::sleep(ms(1500));
            assert_eq!(rx.recv(), Ok(7));
            assert_eq!(rx.recv(), Ok(8));
            assert_eq!(rx.recv(), Ok(9));
        });
    });

    let (tx, rx) = bounded(0);
    let tx = WrappedSender(tx);
    let rx = WrappedReceiver(rx);

    crossbeam::scope(|s| {
        s.spawn(move || {
            assert_eq!(tx.send(7), Ok(()));
            thread::sleep(ms(1000));
            assert_eq!(tx.send(8), Ok(()));
            thread::sleep(ms(1000));
            assert_eq!(tx.send(9), Ok(()));
            assert_eq!(tx.send(10), Err(SendError(10)));
        });
        s.spawn(move || {
            thread::sleep(ms(1500));
            assert_eq!(rx.recv(), Ok(7));
            assert_eq!(rx.recv(), Ok(8));
            assert_eq!(rx.recv(), Ok(9));
        });
    });
}

#[test]
fn send_timeout() {
    let (tx, rx) = bounded(2);
    let tx = WrappedSender(tx);
    let rx = WrappedReceiver(rx);

    crossbeam::scope(|s| {
        s.spawn(move || {
            assert_eq!(tx.send_timeout(1, ms(1000)), Ok(()));
            assert_eq!(tx.send_timeout(2, ms(1000)), Ok(()));
            assert_eq!(
                tx.send_timeout(3, ms(500)),
                Err(SendTimeoutError::Timeout(3))
            );
            thread::sleep(ms(1000));
            assert_eq!(tx.send_timeout(4, ms(1000)), Ok(()));
            thread::sleep(ms(1000));
            assert_eq!(tx.send(5), Err(SendError(5)));
        });
        s.spawn(move || {
            thread::sleep(ms(1000));
            assert_eq!(rx.recv(), Ok(1));
            thread::sleep(ms(1000));
            assert_eq!(rx.recv(), Ok(2));
            assert_eq!(rx.recv(), Ok(4));
        });
    });

    let (tx, rx) = bounded(0);
    let tx = WrappedSender(tx);
    let rx = WrappedReceiver(rx);

    crossbeam::scope(|s| {
        s.spawn(move || {
            assert_eq!(
                tx.send_timeout(7, ms(1000)),
                Err(SendTimeoutError::Timeout(7))
            );
            assert_eq!(tx.send_timeout(8, ms(1000)), Ok(()));
            assert_eq!(
                tx.send_timeout(9, ms(1000)),
                Err(SendTimeoutError::Disconnected(9))
            );
        });
        s.spawn(move || {
            thread::sleep(ms(1500));
            assert_eq!(rx.recv(), Ok(8));
        });
    });
}

#[test]
fn try_send() {
    let (tx, rx) = bounded(1);
    let tx = WrappedSender(tx);
    let rx = WrappedReceiver(rx);

    crossbeam::scope(|s| {
        s.spawn(move || {
            assert_eq!(tx.try_send(1), Ok(()));
            assert_eq!(tx.try_send(2), Err(TrySendError::Full(2)));
            thread::sleep(ms(1500));
            assert_eq!(tx.try_send(3), Ok(()));
            thread::sleep(ms(500));
            assert_eq!(tx.try_send(4), Err(TrySendError::Disconnected(4)));
        });
        s.spawn(move || {
            thread::sleep(ms(1000));
            assert_eq!(rx.try_recv(), Ok(1));
            assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
            assert_eq!(rx.recv(), Ok(3));
        });
    });

    let (tx, rx) = bounded(0);
    let tx = WrappedSender(tx);
    let rx = WrappedReceiver(rx);

    crossbeam::scope(|s| {
        s.spawn(move || {
            assert_eq!(tx.try_send(7), Err(TrySendError::Full(7)));
            thread::sleep(ms(1500));
            assert_eq!(tx.try_send(8), Ok(()));
            thread::sleep(ms(500));
            assert_eq!(tx.try_send(9), Err(TrySendError::Disconnected(9)));
        });
        s.spawn(move || {
            thread::sleep(ms(1000));
            assert_eq!(rx.recv(), Ok(8));
        });
    });
}

#[test]
fn recv_after_close() {
    let (tx, rx) = bounded(100);
    let tx = WrappedSender(tx);
    let rx = WrappedReceiver(rx);

    tx.send(1).unwrap();
    tx.send(2).unwrap();
    tx.send(3).unwrap();

    drop(tx);

    assert_eq!(rx.recv(), Ok(1));
    assert_eq!(rx.recv(), Ok(2));
    assert_eq!(rx.recv(), Ok(3));
    assert_eq!(rx.recv(), Err(RecvError));
}

#[test]
fn matching() {
    let (tx, rx) = channel::bounded(0);
    let (tx, rx) = (&tx, &rx);

    crossbeam::scope(|s| for i in 0..44 {
        s.spawn(move || loop {
            if let Ok(x) = select::recv(rx) {
                assert_ne!(i, x);
                break;
            }
            if let Ok(()) = select::send(tx, i) {
                break;
            }
        });
    });

    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
}

#[test]
fn matching_with_leftover() {
    let (tx, rx) = channel::bounded(1);
    let (tx, rx) = (&tx, &rx);

    crossbeam::scope(|s| {
        for i in 0..55 {
            s.spawn(move || loop {
                if let Ok(x) = select::recv(&rx) {
                    assert_ne!(i, x);
                    break;
                }
                if let Ok(()) = select::send(&tx, i) {
                    break;
                }
            });
        }
        tx.send(!0).unwrap();
    });

    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
}

#[test]
fn channel_through_channel() {
    const COUNT: usize = 1000;

    type T = Box<Any + Send>;

    for cap in 0..3 {
        let (tx, rx) = channel::bounded::<T>(cap);

        crossbeam::scope(|s| {
            s.spawn(move || {
                let mut tx = tx;

                for _ in 0..COUNT {
                    let (new_tx, new_rx) = channel::bounded(cap);
                    let mut new_rx: T = Box::new(Some(new_rx));

                    loop {
                        if let Err(r) = select::send(&tx, new_rx) {
                            new_rx = r;
                        } else {
                            break;
                        }
                    }

                    tx = new_tx;
                }
            });

            s.spawn(move || {
                let mut rx = rx;

                for _ in 0..COUNT {
                    loop {
                        if let Ok(mut r) = select::recv(&rx) {
                            rx = r.downcast_mut::<Option<Receiver<T>>>()
                                .unwrap()
                                .take()
                                .unwrap();
                            break;
                        }
                    }
                }
            });
        });
    }
}
