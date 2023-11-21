# IO-Partition

This is a prototype of an IO-partition for an ARINC 653P4 compliant hypervisor.
The project goal is to explore the possiblilties for network-transparent
inter-partition APEX channels in the context of redundancy management and
dynamic reconfiguration of IMA systems. It is developed in the memory-safe
Rust programming language and uses [a653rs](https://github.com/DLR-FT/a653rs)
to communicate with the hypervisor.

Keep in mind that this is a research project and it has not been developed and
tested according to DO-178C.

## Development

The development environment of this project is managed using [nix](https://nixos.org/download.html#download-nix).
To enter the environment, run `nix develop`.

## Documentation

```
cargo doc --all --open
```

## Nightly features

The echo example and support for Zynq7000 currently require a nightly compiler for support of const-generics.
Other than that all crates should use only stable rust features. 
