use io::Write;
use parking_lot::Mutex;
use stateful::SpinnerCommunicator;
use std::{
    error::Error,
    io::{self, BufRead, BufReader, Read},
    path::Path,
    process::{Command, Output, Stdio},
    sync::{mpsc::channel, mpsc::Receiver, Arc},
    thread,
};

fn child_stream_to_lines<R>(stream: R) -> (Arc<Mutex<Vec<u8>>>, Receiver<()>)
where
    R: Read + Send + 'static,
{
    let out = Arc::new(Mutex::new(vec![]));
    let vec = out.clone();
    let mut reader = BufReader::new(stream);
    let (sender, receiver) = channel::<()>();
    thread::Builder::new()
        .name("child_stream_to_vec".into())
        .spawn(move || {
            loop {
                let mut buf = Vec::with_capacity(1);
                match reader.read_until(b'\n', &mut buf) {
                    Err(e) => {
                        eprintln!("{}}} Error reading from stream: {}", line!(), e);
                        break;
                    }
                    Ok(got) => match got {
                        0 => break,
                        _ => {
                            let mut lock = vec.lock();
                            *lock = buf;
                        }
                    },
                }
            }
            sender.send(()).unwrap_or_else(|_| {});
        })
        .unwrap_or_else(|_| panic!("{}}} Could not spawn thread", line!()));
    (out, receiver)
}

pub fn command_from_str(s: &str) -> Command {
    let split = s.split_whitespace().collect::<Vec<_>>();
    let mut command = Command::new(split[0]);
    command.args(&split[1..]);
    command
}

pub fn run(
    mut command: Command,
    spinner_communicator: Option<&SpinnerCommunicator>,
    status_message: Option<&str>,
    done_message: Option<&str>,
    show_output: bool,
    chdir: Option<&Path>,
) -> Result<Output, Box<dyn Error>> {
    if let Some(cwd) = chdir {
        command.current_dir(cwd);
    }
    if show_output && spinner_communicator.is_some() {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
    }

    let output = match spinner_communicator {
        Some(sc) => {
            if let Some(msg) = status_message {
                sc.send_status_message(msg)?;
            }
            if let Some(msg) = done_message {
                sc.send_done_message(msg)?;
            }
            sc.activate()?;
            let output = _run_internal(command, show_output)?;
            sc.deactivate()?;
            output
        }
        None => {
            if show_output {
                let child = command.spawn()?;
                child.wait_with_output()?
            } else {
                command.output()?
            }
        }
    };

    Ok(output)
}

fn _run_internal(mut command: Command, show_output: bool) -> Result<Output, Box<dyn Error>> {
    let mut child = command.spawn()?;
    if show_output {
        let (out, out_done) = child_stream_to_lines(child.stdout.take().unwrap());
        let (err, err_done) = child_stream_to_lines(child.stderr.take().unwrap());

        loop {
            let mut outl = out.lock();
            let mut errl = err.lock();
            if !outl.is_empty() || !errl.is_empty() {
                let mut stdout = io::stdout();
                stdout.write_all(b"\x1b[2K\x1b[G")?;
                stdout.write_all(&outl)?;
                stdout.write_all(&errl)?;
                stdout.flush()?;
            }
            outl.clear();
            errl.clear();
            if out_done.try_recv().is_ok() {
                // silence "unused" warning
                drop(err_done);
                break;
            }
        }
    }
    let output = child.wait_with_output()?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_run() {
        run(command_from_str("echo test"), None, None, None, true, None).unwrap();

        let spinner_communicator =
            stateful::spawn_spinner(&["/", "-", "\\", "|", "*"], Duration::from_millis(100));

        run(
            command_from_str("sleep 5"),
            Some(&spinner_communicator),
            Some("Waiting for something to happen..."),
            Some("Something definitely happened."),
            true,
            None,
        )
        .unwrap();

        thread::sleep(Duration::from_millis(100));
        spinner_communicator.stop().unwrap();
    }
}
