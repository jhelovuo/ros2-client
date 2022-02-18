use std::{io::Write, os::unix::io::AsRawFd};

#[allow(unused_imports)]
use log::{debug, error, info, warn};
use mio::{unix::EventedFd, Events, Poll, PollOpt, Ready, Token};
use mio_extras::channel as mio_channel;
use termion::{event::Key, input::TermRead, AsyncReader};

use crate::{PenRequest, Pose, Twist, Vector3};

#[derive(Debug)]
pub enum RosCommand {
    StopEventLoop,
    TurtleCmdVel { turtle_id: i32, twist: Twist },
    Reset,
    SetPen(PenRequest),
    Spawn(String),
    Kill(String),
}

// Define turtle movement commands as Twist values
const MOVE_FORWARD: Twist = Twist {
    linear: Vector3 {
        x: 2.0,
        ..Vector3::ZERO
    },
    angular: Vector3::ZERO,
};

const MOVE_BACKWARD: Twist = Twist {
    linear: Vector3 {
        x: -2.0,
        ..Vector3::ZERO
    },
    angular: Vector3::ZERO,
};

const ROTATE_LEFT: Twist = Twist {
    linear: Vector3::ZERO,
    angular: Vector3 {
        z: 2.0,
        ..Vector3::ZERO
    },
};

const ROTATE_RIGHT: Twist = Twist {
    linear: Vector3::ZERO,
    angular: Vector3 {
        z: -2.0,
        ..Vector3::ZERO
    },
};

pub struct UiController {
    poll: Poll,
    stdout: std::io::Stdout,
    async_reader: termion::input::Events<AsyncReader>,
    command_sender: mio_channel::SyncSender<RosCommand>,
    readback_receiver: mio_channel::Receiver<Twist>,
    pose_receiver: mio_channel::Receiver<Pose>,
}

impl UiController {
    const KEYBOARD_CHECK_TOKEN: Token = Token(0);
    const READBACK_TOKEN: Token = Token(1);
    const POSE_TOKEN: Token = Token(2);

    pub fn new(
        stdout: std::io::Stdout,
        command_sender: mio_channel::SyncSender<RosCommand>,
        readback_receiver: mio_channel::Receiver<Twist>,
        pose_receiver: mio_channel::Receiver<Pose>,
    ) -> UiController {
        let poll = Poll::new().unwrap();
        let async_reader = termion::async_stdin().events();

        UiController {
            poll,
            stdout,
            async_reader,
            command_sender,
            readback_receiver,
            pose_receiver,
        }
    }

    pub fn start(&mut self) {
        ctrlc::set_handler(move || {
            println!("Aborting");
            std::process::abort();
        })
        .expect("Error setting Ctrl-C handler");

        self.poll
            .register(
                &EventedFd(&std::io::stdin().lock().as_raw_fd()),
                UiController::KEYBOARD_CHECK_TOKEN,
                Ready::readable(),
                PollOpt::level(),
            )
            .unwrap();

        self.poll
            .register(
                &self.readback_receiver,
                UiController::READBACK_TOKEN,
                Ready::readable(),
                PollOpt::edge(),
            )
            .unwrap();
        self.poll
            .register(
                &self.pose_receiver,
                UiController::POSE_TOKEN,
                Ready::readable(),
                PollOpt::edge(),
            )
            .unwrap();

        // clearing screen
        write!(
            self.stdout,
            "{}{}Press q to quit, cursor keys to control turtle.",
            termion::clear::All,
            termion::cursor::Goto(1, 1)
        )
        .unwrap();
        self.stdout.flush().unwrap();

        let mut turtle_id = 1;

        let mut pen_index = 0;
        let pen_requests = vec![
            PenRequest {
                r: 255,
                b: 0,
                g: 0,
                width: 3,
                off: 0,
            },
            PenRequest {
                r: 255,
                b: 0,
                g: 200,
                width: 5,
                off: 0,
            },
            PenRequest {
                r: 250,
                b: 250,
                g: 250,
                width: 2,
                off: 1,
            },
            PenRequest {
                r: 0,
                b: 0,
                g: 250,
                width: 1,
                off: 0,
            },
            PenRequest {
                r: 0,
                b: 0,
                g: 0,
                width: 1,
                off: 0,
            },
        ];

        loop {
            write!(self.stdout, "{}", termion::cursor::Goto(1, 1)).unwrap();
            self.stdout.flush().unwrap();

            let mut events = Events::with_capacity(100);
            self.poll.poll(&mut events, None).unwrap();

            for event in events.iter() {
                if event.token() == UiController::KEYBOARD_CHECK_TOKEN {
                    // a small wait here to allow the termion input mechnism to react.
                    // Still some keyboard presses are missed. What are we doing wrong here?
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    while let Some(Ok(termion::event::Event::Key(key))) = &self.async_reader.next()
                    {
                        write!(
                            self.stdout,
                            "{}{}{:?}",
                            termion::cursor::Goto(1, 2),
                            termion::clear::CurrentLine,
                            key,
                        )
                        .unwrap();
                        info!("key: {:?}", key);
                        match key {
                            Key::Char('q') | Key::Ctrl('c') => {
                                debug!("Quit.");
                                self.send_command(RosCommand::StopEventLoop);
                                return; // stop loop
                            }
                            Key::Char('r') => {
                                debug!("Reset request");
                                self.send_command(RosCommand::Reset);
                            }
                            Key::Char('p') => {
                                debug!("Pen request");
                                self.send_command(RosCommand::SetPen(
                                    pen_requests[pen_index].clone(),
                                ));
                                pen_index = (pen_index + 1) % pen_requests.len();
                            }
                            Key::Char('a') => {
                                debug!("Spawn 1");
                                self.send_command(RosCommand::Spawn("turtle1".to_owned()));
                            }
                            Key::Char('b') => {
                                debug!("Spawn 2");
                                self.send_command(RosCommand::Spawn("turtle2".to_owned()));
                            }
                            Key::Char('A') => {
                                debug!("Kill 1");
                                self.send_command(RosCommand::Kill("turtle1".to_owned()));
                            }
                            Key::Char('B') => {
                                debug!("Kill 2");
                                self.send_command(RosCommand::Kill("turtle2".to_owned()));
                            }

                            Key::Char('1') => {
                                turtle_id = 1;
                            }

                            Key::Char('2') => {
                                turtle_id = 2;
                            }

                            Key::Up => {
                                debug!("Move left.");
                                let twist = MOVE_FORWARD;
                                self.print_sent_turtle_cmd_vel(&twist);
                                self.send_command(RosCommand::TurtleCmdVel { turtle_id, twist })
                            }
                            Key::Right => {
                                debug!("Move right.");
                                let twist = ROTATE_RIGHT;
                                self.print_sent_turtle_cmd_vel(&twist);
                                self.send_command(RosCommand::TurtleCmdVel { turtle_id, twist })
                            }
                            Key::Down => {
                                debug!("Rotate down.");
                                let twist = MOVE_BACKWARD;
                                self.print_sent_turtle_cmd_vel(&twist);
                                self.send_command(RosCommand::TurtleCmdVel { turtle_id, twist })
                            }
                            Key::Left => {
                                debug!("Rotate left.");
                                let twist = ROTATE_LEFT;
                                self.print_sent_turtle_cmd_vel(&twist);
                                self.send_command(RosCommand::TurtleCmdVel { turtle_id, twist })
                            }
                            _ => (),
                        }
                    }
                } else if event.token() == UiController::READBACK_TOKEN {
                    while let Ok(twist) = self.readback_receiver.try_recv() {
                        write!(
                            self.stdout,
                            "{}{}Read Turtle cmd_vel {:?}",
                            termion::cursor::Goto(1, 6),
                            termion::clear::CurrentLine,
                            twist
                        )
                        .unwrap();
                    }
                } else if event.token() == UiController::POSE_TOKEN {
                    while let Ok(pose) = self.pose_receiver.try_recv() {
                        write!(
                            self.stdout,
                            "{}{}Turtle pose {:?}",
                            termion::cursor::Goto(1, 8),
                            termion::clear::CurrentLine,
                            pose
                        )
                        .unwrap();
                    }
                } else {
                    error!("What is this? {:?}", event.token())
                }
            }
        }
    }

    fn send_command(&self, command: RosCommand) {
        self.command_sender
            .try_send(command)
            .unwrap_or_else(|e| error!("UI: Failed to send command {:?}", e))
    }

    fn print_sent_turtle_cmd_vel(&mut self, twist: &Twist) {
        write!(
            self.stdout,
            "{}{}Sent Turtle cmd_vel {:?}",
            termion::cursor::Goto(1, 4),
            termion::clear::CurrentLine,
            twist
        )
        .unwrap();
    }
}
