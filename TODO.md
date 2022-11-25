# TODO

- use ListMap in route table instead of Vec
- test network port that just stores and traces all frames
- test network port that can be programmed to receive frames for testing 
- traffic shaper (credit based)
- inter-partition communication via VL
- network interface layer
- queueing ports

## Open problems

### Configure partition with configuration contents
- ports can not be configured without knowing the message sizes in advance
- should not require a YAML-Parser as part of the compiled partition
- ports should be able to be agnostic of message content

#### Solutions
- proc-macro to generate ports with constant generic parameters from config
- (could use unchecked sampling_port_send / receive)
- for prototype: statically define a set of ports of fixed sizes
  - configure based on settings in config

### Forwarding from one sampling-port to another
- data validity / age is attached to sampling port -> has to be signaled in-band?
- can not be guaranteed for sampling ports over network, because of network delay that is unknown to receiving hypervisor