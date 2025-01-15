pub mod test_data {
    pub const CFG: &str = r##"
period:
  secs: 1
  nanos: 0
time_capacity:
  secs: 0
  nanos: 300000
stack_size: 10000
virtual_links:
  1:
    period:
      secs: 0
      nanos: 10000
    source: "EchoRequest"
    destinations: [ "NodeB" ]
  2:
    period:
      secs: 0
      nanos: 10000
    source: "NodeB"
    destinations: [ "EchoReply" ]
ports:
  EchoRequest:
    !sampling_in
    msg_size: 1000
    refresh_period:
      secs: 10
      nanos: 0
  EchoReply:
    !sampling_out
    msg_size: 1000    
interfaces:
  NodeB:
    destination: "192.168.1.2:8082"
    mtu: 1500
    rate: 10000000
    source: "0.0.0.0:8081"
"##;
}

use a653rs::bindings::{
    ApexPartitionP4, ApexPartitionStatus, ApexProcessP4, ApexQueuingPortP4, ApexSamplingPortP4,
    ApexSystemTime, ApexTimeP4, MessageRange, MessageSize, OperatingMode, ProcessId, QueueOverflow,
    QueuingPortId, Validity,
};
use a653rs_router::prelude::{
    CreateNetworkInterfaceId, NetworkInterfaceId, PlatformNetworkInterface,
};

#[derive(Debug)]
pub struct DummyHypervisor;

impl ApexQueuingPortP4 for DummyHypervisor {
    fn create_queuing_port(
        _queuing_port_name: a653rs::bindings::QueuingPortName,
        _max_message_size: a653rs::prelude::MessageSize,
        _max_nb_message: a653rs::prelude::MessageRange,
        _port_direction: a653rs::bindings::PortDirection,
        _queuing_discipline: a653rs::prelude::QueuingDiscipline,
    ) -> Result<a653rs::prelude::QueuingPortId, a653rs::bindings::ErrorReturnCode> {
        Ok(QueuingPortId::from(1))
    }

    fn send_queuing_message(
        _queuing_port_id: a653rs::prelude::QueuingPortId,
        _message: &[a653rs::prelude::ApexByte],
        _time_out: a653rs::bindings::ApexSystemTime,
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        Ok(())
    }

    unsafe fn receive_queuing_message(
        _queuing_port_id: a653rs::prelude::QueuingPortId,
        _time_out: a653rs::bindings::ApexSystemTime,
        _message: &mut [a653rs::prelude::ApexByte],
    ) -> Result<
        (a653rs::prelude::MessageSize, a653rs::prelude::QueueOverflow),
        a653rs::bindings::ErrorReturnCode,
    > {
        Ok((MessageSize::from(1u32), QueueOverflow::from(false)))
    }

    fn get_queuing_port_status(
        _queuing_port_id: a653rs::prelude::QueuingPortId,
    ) -> Result<a653rs::prelude::QueuingPortStatus, a653rs::bindings::ErrorReturnCode> {
        Ok(a653rs::bindings::QueuingPortStatus {
            nb_message: MessageRange::from(1u32),
            max_nb_message: MessageRange::from(10u32),
            max_message_size: MessageSize::from(1000u32),
            port_direction: a653rs::bindings::PortDirection::Source,
            waiting_processes: 0,
        })
    }

    fn clear_queuing_port(
        _queuing_port_id: a653rs::prelude::QueuingPortId,
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        Ok(())
    }
}

impl ApexSamplingPortP4 for DummyHypervisor {
    fn create_sampling_port(
        _sampling_port_name: a653rs::bindings::SamplingPortName,
        _max_message_size: a653rs::prelude::MessageSize,
        _port_direction: a653rs::bindings::PortDirection,
        _refresh_period: a653rs::bindings::ApexSystemTime,
    ) -> Result<a653rs::prelude::SamplingPortId, a653rs::bindings::ErrorReturnCode> {
        Ok(1)
    }

    fn write_sampling_message(
        _sampling_port_id: a653rs::prelude::SamplingPortId,
        _message: &[a653rs::prelude::ApexByte],
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        Ok(())
    }

    unsafe fn read_sampling_message(
        _sampling_port_id: a653rs::prelude::SamplingPortId,
        _message: &mut [a653rs::prelude::ApexByte],
    ) -> Result<
        (a653rs::prelude::Validity, a653rs::prelude::MessageSize),
        a653rs::bindings::ErrorReturnCode,
    > {
        Ok((Validity::Valid, MessageSize::from(1u32)))
    }
}

impl ApexTimeP4 for DummyHypervisor {
    fn periodic_wait() -> Result<(), a653rs::bindings::ErrorReturnCode> {
        Ok(())
    }

    fn get_time() -> a653rs::bindings::ApexSystemTime {
        ApexSystemTime::from(1)
    }
}

impl ApexPartitionP4 for DummyHypervisor {
    fn get_partition_status() -> a653rs::bindings::ApexPartitionStatus {
        ApexPartitionStatus {
            period: 10,
            duration: 10,
            identifier: 10,
            lock_level: 10,
            operating_mode: OperatingMode::ColdStart,
            start_condition: a653rs::bindings::StartCondition::NormalStart,
            num_assigned_cores: 1,
        }
    }

    fn set_partition_mode(
        _operating_mode: a653rs::prelude::OperatingMode,
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        Ok(())
    }
}

impl ApexProcessP4 for DummyHypervisor {
    fn create_process(
        _attributes: &a653rs::bindings::ApexProcessAttribute,
    ) -> Result<a653rs::prelude::ProcessId, a653rs::bindings::ErrorReturnCode> {
        Ok(ProcessId::from(1))
    }

    fn start(
        _process_id: a653rs::prelude::ProcessId,
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct DummyNetIntf;

impl PlatformNetworkInterface for DummyNetIntf {
    fn platform_interface_send_unchecked(
        _id: NetworkInterfaceId,
        _buffer: &[u8],
    ) -> Result<usize, a653rs_router::prelude::InterfaceError> {
        Ok(1)
    }

    fn platform_interface_receive_unchecked(
        _id: NetworkInterfaceId,
        buffer: &'_ mut [u8],
    ) -> Result<&'_ [u8], a653rs_router::prelude::InterfaceError> {
        Ok(buffer)
    }
}

impl CreateNetworkInterfaceId<DummyNetIntf> for DummyNetIntf {
    fn create_network_interface_id(
        _cfg: &a653rs_router::prelude::InterfaceConfig,
    ) -> Result<NetworkInterfaceId, a653rs_router::prelude::InterfaceError> {
        Ok(NetworkInterfaceId(1u32))
    }
}
