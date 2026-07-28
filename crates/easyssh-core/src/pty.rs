use crate::domain::Connection;
use crate::openssh::{OpenSsh, OpenSshError, SshInvocation};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver};
use std::thread;

/// A real PTY attached directly to the system `ssh` child process.
///
/// On Windows `portable-pty` selects its ConPTY backend; Unix uses a native
/// pseudo-terminal. No intermediary shell, agent, or credential prompt is
/// introduced by this type.
pub struct TerminalSession {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    output: Receiver<Vec<u8>>,
}

impl TerminalSession {
    pub fn connect(
        openssh: &OpenSsh,
        connection: &Connection,
        cols: u16,
        rows: u16,
    ) -> Result<Self, OpenSshError> {
        let invocation = SshInvocation::for_connection(openssh, connection)?;
        Self::spawn(invocation, cols, rows)
    }

    pub fn spawn(invocation: SshInvocation, cols: u16, rows: u16) -> Result<Self, OpenSshError> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| OpenSshError::Failed(error.to_string()))?;
        let mut command = CommandBuilder::new(invocation.executable);
        command.args(&invocation.args);
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| OpenSshError::Failed(error.to_string()))?;
        drop(pair.slave);
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| OpenSshError::Failed(error.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| OpenSshError::Failed(error.to_string()))?;
        let (sender, output) = mpsc::channel();
        thread::spawn(move || {
            let mut buffer = [0_u8; 8192];
            while let Ok(count) = reader.read(&mut buffer) {
                if count == 0 || sender.send(buffer[..count].to_vec()).is_err() {
                    break;
                }
            }
        });
        Ok(Self {
            master: pair.master,
            child,
            writer,
            output,
        })
    }

    pub fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }
    pub fn resize(&mut self, cols: u16, rows: u16) -> std::io::Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(std::io::Error::other)
    }
    pub fn try_read(&self) -> Option<Vec<u8>> {
        self.output.try_recv().ok()
    }
    pub fn is_alive(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }
    pub fn close(&mut self) -> std::io::Result<()> {
        self.child.kill()
    }
}
