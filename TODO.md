# TODO

## Do
- test network port that just stores and traces all frames
- tracing with uprobes
- network interface layer
- support for queueing ports
- better test coverage

## Maybe
- use SKE in CI for testing with XNG
- requirements testing with Cucumber

## Open Problems

### Configure partition with configuration contents
- ports can not be configured without knowing the message sizes in advance
- should not require a YAML-Parser as part of the compiled partition
- proc-macro to generate ports with constant generic parameters from config
- for prototype: statically define a set of ports of fixed sizes
  - configure based on settings in config

### Forwarding of sampling-ports over the network
- data validity / age is attached to sampling port -> has to be signaled in-band?
- can not be guaranteed for sampling ports over network, because of network delay that is unknown to receiving hypervisor
  - network delay is bounded -> can be used by receiving hypervisor
