# IO-Partition

This is a prototype of an IO-partition for an ARINC 653P4 compliant hypervisor.
The project goal is to explore the possiblilties for network-transparent
inter-partition APEX channels in the context of redundancy management and
dynamic reconfiguration of IMA systems. It is developed in the memory-safe
Rust programming language and uses [apex-rs](https://github.com/aeronautical-
informatics/apex-rs) to communicate with the hypervisor.

Keep in mind that this is a research project and it has not been developed and
tested according to DO-178C.

## Virtual Links

The purpose of the IO-partition is to transparently bridge APEX channels over
the network and to make their contents available to partitions running on the
same hypervisor and on other distributed hypervisors. The IO-partition provides
single-producer-multiple-consumer extensions of APEX channels locally and over
the network as a building-block for a distributed redundancy-management. No
code-changes are required for partitions to interact with the IO partition.
Instead, the hypervisor's configuration can be adapted by replacing each direct
channel between two or more partitions with channels between the partitions and
the IO-partition. Each such resulting virtual link between multiple partitions
is unique inside the deployment. Each source port can only be part of at-most
one virtual link, which allows to uniquely identify the originator of a message
based on the message's virtual link id.

## Networking

Optionally, each virtual link may have one or more network interfaces among
its destinations, or source its data from the network instead of from a local
partition. The IO-partition uses a plugable interface for communicating with
the concrete network interface driver. This way, the driver's implementation can
be allowed to use platform-dependent features, while the IO-partition remains
portable to other hypervisors and platforms. The driver can be specified at
compile time and there are two implementations available:

### `Udp`

This driver sends and receives virtual links via one or more UDP network
sockets. It requires the [apex-linux](https://github.com/aeronautical-informatics/apex-linux)
hypervisor, which can create these sockets and pass them to the IO-partition.
This way, the otherwise strictly sandboxed IO-partition gains limited
network-access.

### `Uart`

Communicates via memory-mapped IO with an UART FGPA-core. The framing  is done
in software using COBS encoding and a checksum for error detection. It requires
a hypervisor that can delegate additional memory regions to the IO-partition.

## Scheduling

The scheduler implementation governs the manner by which the virtual links are
sampled, forwarded to their destination channels and optionally multiplexed to
the network. Like the network driver implementation, the implementation of the
scheduler can also be exchanged at compile time without requiring changes to
the IO-partition's code. The scheduler may use any method of determining which
virtual link shall be sampled next. The available implementations are:

### `DeadlineRR`

A deadline-based round-robin scheduler, that attempts to limit the jitter of
the messages transmitted on each virtual link and restrict the utilisation of
the attached network interfaces.

## Configuration

Configuration is done at compile-time using a YAML configuration file as an
argument to the build script. The build script generates code that initializes
the run-time state of the partition such as ports, interfaces and the scheduler.
This way, that the IO-partition does not rely on heap-allocated memory during
run-time; the exact memory requirements are known at compile-time and are
tracable to the configuration. Examples of the configuration file format are
contained in the [config](./config) directory.

## Development

The development environment of this project is managed using [nix](https://nixos.org/download.html#download-nix).
To enter the environment, run `nix develop`.

## Running the examples

There are multiple runnable examples using different hypervisors. The examples
use two IO partition on distributed hypervisors to communicate a partition
running an echo client and one running an echo server. The client will send an
echo request using virtual link 1 and the server, after processing the message,
will respond with an echo reply using virtual link 2. Run either `test-run-
echo-scoped` to run the example using apex-linux and UDP sockets, or `test-run-
echo-cora` to run the example using LithOS on XNG on two CoraZ7 boards that are
connected using UART.

## Logging

The IO-partition uses Rust's standard log facility which, if not explicitly
configured comes at no run-time overhead. If log output is desired (e.g. for
debugging purposes), a platform-dependent logger implementation needs to be
provided. Both apex-linux and this repo provide loggers that are targeted for
use with specific hypervisors:

`apex_rs_linux::partition::ApexLogger` logs message to the health manager
of apex-linux, which will print them to the standard output, together with the
output of the hypervisor and the output of the other partitions.

`coraz7::XalLogger` logs messages to the XNG console using XalPutChar. Using
this logger requires that the XAL is linked into the final partition image in
addition LithOS.

## Profiling

The crates inside this workspace make use of a common trace facility. The
`coraz7` crate provides an implementation of this facility that allows log the
traced events and their associated data to GPIO ports. This is useful, because
this way a logic analyzer can be used to record both the trace events and
e.g. network communication on the UART using the same clock. In practice, this
allows a precision below 1µs, which is good enough for quantifying the delays
introduced by the IO-partition.

## Planned features

### Runtime reconfiguration

Currently, most of the initialization code is generated based of the contents
of the configuration file. It should be possible present the build script with
a number of alternative configurations from which it can derive the maximum size
of all data-structures. The active configuration (and possible alternatives)
could then be passed to the IO partition at run-time (e.g. during the partition
cold-start) and the contents of the data-structures can be exchanged to fit
the new configuration. This would also loosen the requirement for recompiling
the partition on configuration changes, which would probably greatly help
certifiablilty.

### Rewrite using embedded-hal

The embedded-hal provides interfaces for platform-specific implementations of
device drivers and registers. It would be nice to be able to make use of these
interfaces to able to use crates that target these interfaces.

### Use the Instrumentation Trace Macrocell (ITM) for profiling

ARMv7m has support for low-level trace points that can write into a small memory
buffer. Should the GPIO-tracing have too much overhead or be too imprecise, this
would likely be a better solution.
