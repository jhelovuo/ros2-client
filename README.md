# ROS2 Client

This is a Rust native client library for Rust. It does not link to rclcpp, or any non-Rust DDS library. RustDDS is used for communication.

# Architecture Ideas

The intation is that this is a medium-level library between lower-level DDS. However, it does not attempt to provide higher-level services, such as an event loop, or Actions. These are to be implemented as a separate crate.

# Example: turtle_teleop

The included example program shoudl be able to communicate with out-of-the-box ROS2 turtlesim example.

Teleop example program currentlt has the following keyboard commands:

* Cursor keys: Move turtle
* `q` or `Ctrl-C`: quit
* `r`: reset simulator
* `p`: change pen color
* `a`/`b` : spawn turtle1 / turtle2
* `A`/`B` : kill turtle1 / turtle2
* `1`/`2` : switch control between turtle1 / turtle2
