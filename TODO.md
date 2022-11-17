# TODO

- configure partition with configuration contents
  - should not require a YAML-Parser as part of the compiled partition
  - maybe generate code from YAML config?
- ports can not be configured without knowing the message sizes in advance
  - also: ports should be able to be agnostic of message content
  - could use unchecked sampling_port_send / receive
  - for prototype: statically define a set of ports of fixed sizes
    - configure based on settings in config
  - later: use proc-macros to statically define set of ports with fixed sizes
    - can have individual sizes?
- forwarding from one sampling-port to another
  - data validity / age is attached to sampling port -> has to be signaled in-band?
  - can not be guaranteed for sampling ports over network, because of network delay that is unknown to receiving hypervisor
