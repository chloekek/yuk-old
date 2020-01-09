use futures::executor::ThreadPool;
use futures::task::SpawnExt;
use std::error::Error;
use std::io;
use yuk::io::dispatch::Dispatcher;

fn main() -> Result<(), Box<dyn Error>>
{
    let executor = ThreadPool::new()?;

    let d = Dispatcher::new(/* epoll_cloexec */ true)?;

    executor.spawn_with_handle(async_main(d))?;

    loop {
        d.poll()?;
    }
}

async fn async_main(d: Dispatcher) -> io::Result<()>
{
    d.subscribe(0)?;
    Ok(())
}
