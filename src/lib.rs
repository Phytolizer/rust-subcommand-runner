use chrono::Utc;
use crossbeam_channel::Sender;
use stateful::run_with_status;
use std::{error::Error, path::Path, process::{Output, Command}};

pub fn cd_and_run<S, P>(
    command: S,
    chdir: P,
    message_sender: &Sender<String>,
    done_message_sender: &Sender<String>,
    start_sender: &Sender<()>,
    done_sender: &Sender<()>,
) -> Result<Output, Box<dyn Error>>
where
    S: AsRef<str>,
    P: AsRef<Path>,
{
    let command = command.as_ref();
    let command_parts: Vec<_> = command.split_whitespace().collect();

    let mut cmd_to_run = Command::new(&command_parts[0]);
    cmd_to_run
        .args(&command_parts[1..])
        .current_dir(chdir.as_ref());
    Ok(run_command(
        cmd_to_run,
        command,
        message_sender,
        done_message_sender,
        start_sender,
        done_sender,
    )?)
}

pub fn cd_and_run_no_spinner<S: AsRef<str>, P: AsRef<Path>>(
    command: S,
    chdir: P,
) -> Result<Output, Box<dyn Error>> {
    let command = command.as_ref();
    let command_parts: Vec<_> = command.split_whitespace().collect();

    let mut cmd_to_run = Command::new(command_parts[0]);
    cmd_to_run.args(&command_parts[1..]).current_dir(chdir);
    Ok(run_command_no_spinner(cmd_to_run, command)?)
}

pub fn run<S>(
    command: S,
    message_sender: &Sender<String>,
    done_message_sender: &Sender<String>,
    start_sender: &Sender<()>,
    done_sender: &Sender<()>,
) -> Result<(), Box<dyn Error>>
where
    S: AsRef<str>,
{
    let command = command.as_ref();
    let command_parts: Vec<_> = command.split_whitespace().collect();

    let mut cmd_to_run = Command::new(command_parts[0]);
    cmd_to_run.args(&command_parts[1..]);
    run_command(
        cmd_to_run,
        command,
        message_sender,
        done_message_sender,
        start_sender,
        done_sender,
    )?;
    Ok(())
}

pub fn run_no_spinner<S: AsRef<str>>(command: S) -> Result<(), Box<dyn Error>> {
    let command = command.as_ref();
    let command_parts: Vec<_> = command.split_whitespace().collect();

    let mut cmd_to_run = Command::new(command_parts[0]);
    cmd_to_run.args(&command_parts[1..]);
    run_command_no_spinner(cmd_to_run, command)?;
    Ok(())
}

pub fn run_command<S: AsRef<str>>(
    command: Command,
    command_str: S,
    message_sender: &Sender<String>,
    done_message_sender: &Sender<String>,
    start_sender: &Sender<()>,
    done_sender: &Sender<()>,
) -> Result<Output, Box<dyn Error>> {
    println!(">> {}", command_str.as_ref());
    let start_time = Utc::now();
    message_sender.send(command_str.as_ref().to_string())?;
    done_message_sender.send(command_str.as_ref().to_string())?;
    start_sender.send(())?;
    let result = run_with_status(command, done_sender, true)?;
    let time_elapsed = Utc::now() - start_time;
    println!(
        "\x1b[2K\x1b[G> Time elapsed: {:?}",
        time_elapsed.to_std().unwrap()
    );
    Ok(result)
}

pub fn run_command_no_spinner<S: AsRef<str>>(
    mut command: Command,
    command_str: S,
) -> Result<Output, Box<dyn Error>> {
    println!(">> {}", command_str.as_ref());
    let start_time = Utc::now();
    let child = command.spawn()?;
    let result = child.wait_with_output()?;
    let time_elapsed = Utc::now() - start_time;
    println!(
        "\x1b[2K\x1b[G> Time elapsed: {:?}",
        time_elapsed.to_std().unwrap()
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
