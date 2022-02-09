# ROS2 Client

This is a Rust native client library for [ROS2](https://docs.ros.org/en/galactic/index.html). It does not link to [rclcpp](https://docs.ros2.org/galactic/api/rclcpp/index.html), or any non-Rust DDS library. RustDDS is used for communication.

## Architecture Ideas

The intetion is that this is a medium-level library between lower-level DDS. However, it does not attempt to provide higher-level services, such as an event loop, or Actions. These are to be implemented as a separate crate on top of this one.

## Example: turtle_teleop

The included example program should be able to communicate with out-of-the-box ROS2 turtlesim example.

![Turtlesim screenshot](examples/turtle_teleop/screenshot.png)

Teleop example program currently has the following keyboard commands:

* Cursor keys: Move turtle
* `q` or `Ctrl-C`: quit
* `r`: reset simulator
* `p`: change pen color
* `a`/`b` : spawn turtle1 / turtle2
* `A`/`B` : kill turtle1 / turtle2
* `1`/`2` : switch control between turtle1 / turtle2

## Status

This is a work-in-progress.

E.g. Service Requests mostly work, but Responses only randomly.

## Acknowledgements

This crate is developed and open-source licensec by [Atostek Oy](https://www.atostek.com/).
