use crossbeam_channel::Sender;
use io::Write;
use parking_lot::Mutex;
use std::{
    error::Error,
    io::{self, BufRead, BufReader, Read},
    path::Path,
    process::{Command, Output, Stdio},
    sync::Arc,
    thread,
};

#[derive(Copy, Clone)]
pub struct SpinnerVars<'sp, S>
where
    S: AsRef<str>,
{
    status_message_sender: &'sp Sender<String>,
    status_message_override: Option<S>,
    done_message_sender: &'sp Sender<String>,
    done_message_override: Option<S>,
    activate_sender: &'sp Sender<()>,
    deactivate_sender: &'sp Sender<()>,
}

impl<'sp, S> SpinnerVars<'sp, S>
where
    S: AsRef<str>,
{
    pub fn new(
        status_message_sender: &'sp Sender<String>,
        status_message_override: Option<S>,
        done_message_sender: &'sp Sender<String>,
        done_message_override: Option<S>,
        activate_sender: &'sp Sender<()>,
        deactivate_sender: &'sp Sender<()>,
    ) -> Self {
        Self {
            status_message_sender,
            status_message_override,
            done_message_sender,
            done_message_override,
            activate_sender,
            deactivate_sender,
        }
    }
}

fn child_stream_to_lines<R>(stream: R) -> Arc<Mutex<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    let out = Arc::new(Mutex::new(vec![]));
    let vec = out.clone();
    let mut reader = BufReader::new(stream);
    thread::Builder::new()
        .name("child_stream_to_vec".into())
        .spawn(move || loop {
            let mut buf = Vec::with_capacity(1);
            match reader.read_until(b'\n', &mut buf) {
                Err(e) => {
                    eprintln!("{}}} Error reading from stream: {}", line!(), e);
                    break;
                }
                Ok(got) => match got {
                    0 => break,
                    _ => {
                        dbg!(&buf);
                        *vec.lock() = buf;
                        dbg!(&vec);
                    }
                },
            }
        })
        .unwrap_or_else(|_| panic!("{}}} Could not spawn thread", line!()));
    out
}

pub fn run<S, P>(
    command_name: S,
    spinner_vars: Option<SpinnerVars<S>>,
    command: Option<Command>,
    show_output: bool,
    cwd: Option<P>,
) -> Result<Output, Box<dyn Error>>
where
    S: AsRef<str>,
    P: AsRef<Path>,
{
    let command_name = command_name.as_ref();
    let command_and_args: Vec<_> = command_name.split_whitespace().collect();
    let mut command = command.unwrap_or_else(|| {
        let mut command = Command::new(command_and_args[0]);
        command.args(&command_and_args[1..]);
        command
    });

    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    let output = match spinner_vars {
        Some(sv) => {
            sv.status_message_sender.send(
                sv.status_message_override
                    .map(|s| s.as_ref().to_string())
                    .unwrap_or_else(|| command_name.to_string()),
            )?;
            sv.done_message_sender.send(
                sv.done_message_override
                    .map(|s| s.as_ref().to_string())
                    .unwrap_or_else(|| command_name.to_string()),
            )?;
            sv.activate_sender.send(())?;
            let output = _run_internal(command, show_output)?;
            sv.deactivate_sender.send(())?;
            output
        }
        None => _run_internal(command, show_output)?,
    };

    Ok(output)
}

fn _run_internal(mut command: Command, show_output: bool) -> Result<Output, Box<dyn Error>> {
    let mut child = command.spawn()?;
    if show_output {
        let out = child_stream_to_lines(child.stdout.take().unwrap());
        let err = child_stream_to_lines(child.stderr.take().unwrap());

        while child.try_wait().is_err() {
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
        }
    }
    let output = child.wait_with_output()?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
