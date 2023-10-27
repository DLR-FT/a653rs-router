#![no_std]
#![allow(unused_variables)]

use a653rs::bindings::*;
use a653rs::prelude::Partition;
use a653rs_router::error::*;
use a653rs_router::prelude::{
    CreateNetworkInterfaceId, InterfaceConfig, PlatformNetworkInterface, Scheduler,
};

pub struct DummyHypervisor;

impl Partition<DummyHypervisor> for DummyHypervisor {
    fn cold_start(&self, ctx: &mut a653rs::prelude::StartContext<DummyHypervisor>) {
        todo!()
    }

    fn warm_start(&self, ctx: &mut a653rs::prelude::StartContext<DummyHypervisor>) {
        todo!()
    }
}

impl ApexPartitionP4 for DummyHypervisor {
    fn get_partition_status<L: a653rs::Locked>() -> a653rs::bindings::ApexPartitionStatus {
        todo!("It works!")
    }

    fn set_partition_mode<L: a653rs::Locked>(
        operating_mode: OperatingMode,
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        todo!()
    }
}

impl ApexSamplingPortP4 for DummyHypervisor {
    fn create_sampling_port<L: a653rs::Locked>(
        sampling_port_name: a653rs::bindings::SamplingPortName,
        max_message_size: MessageSize,
        port_direction: a653rs::bindings::PortDirection,
        refresh_period: a653rs::bindings::ApexSystemTime,
    ) -> Result<SamplingPortId, a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn write_sampling_message<L: a653rs::Locked>(
        sampling_port_id: SamplingPortId,
        message: &[ApexByte],
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    unsafe fn read_sampling_message<L: a653rs::Locked>(
        sampling_port_id: SamplingPortId,
        message: &mut [ApexByte],
    ) -> Result<(Validity, MessageSize), a653rs::bindings::ErrorReturnCode> {
        todo!()
    }
}

impl ApexQueuingPortP4 for DummyHypervisor {
    fn create_queuing_port<L: a653rs::Locked>(
        queuing_port_name: a653rs::bindings::QueuingPortName,
        max_message_size: MessageSize,
        max_nb_message: MessageRange,
        port_direction: a653rs::bindings::PortDirection,
        queuing_discipline: QueuingDiscipline,
    ) -> Result<QueuingPortId, a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn send_queuing_message<L: a653rs::Locked>(
        queuing_port_id: QueuingPortId,
        message: &[ApexByte],
        time_out: a653rs::bindings::ApexSystemTime,
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    unsafe fn receive_queuing_message<L: a653rs::Locked>(
        queuing_port_id: QueuingPortId,
        time_out: a653rs::bindings::ApexSystemTime,
        message: &mut [ApexByte],
    ) -> Result<MessageSize, a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn get_queuing_port_status<L: a653rs::Locked>(
        queuing_port_id: QueuingPortId,
    ) -> Result<QueuingPortStatus, a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn clear_queuing_port<L: a653rs::Locked>(
        queuing_port_id: QueuingPortId,
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        todo!()
    }
}

impl ApexSamplingPortP1 for DummyHypervisor {
    fn get_sampling_port_id<L: a653rs::Locked>(
        sampling_port_name: a653rs::bindings::SamplingPortName,
    ) -> Result<SamplingPortId, a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn get_sampling_port_status<L: a653rs::Locked>(
        sampling_port_id: SamplingPortId,
    ) -> Result<a653rs::bindings::ApexSamplingPortStatus, a653rs::bindings::ErrorReturnCode> {
        todo!()
    }
}

impl ApexQueuingPortP1 for DummyHypervisor {
    fn get_queuing_port_id<L: a653rs::Locked>(
        queuing_port_name: a653rs::bindings::QueuingPortName,
    ) -> Result<QueuingPortId, a653rs::bindings::ErrorReturnCode> {
        todo!()
    }
}

impl ApexProcessP4 for DummyHypervisor {
    fn create_process<L: a653rs::Locked>(
        attributes: &a653rs::bindings::ApexProcessAttribute,
    ) -> Result<ProcessId, a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn start<L: a653rs::Locked>(
        process_id: ProcessId,
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        todo!()
    }
}

impl ApexTimeP4 for DummyHypervisor {
    fn periodic_wait() -> Result<(), a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn get_time() -> i64 {
        todo!()
    }
}

impl ApexProcessP1 for DummyHypervisor {
    fn set_priority<L: a653rs::Locked>(
        process_id: ProcessId,
        priority: Priority,
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn suspend_self<L: a653rs::Locked>(
        time_out: a653rs::bindings::ApexSystemTime,
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn suspend<L: a653rs::Locked>(
        process_id: ProcessId,
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn resume<L: a653rs::Locked>(
        process_id: ProcessId,
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn stop_self<L: a653rs::Locked>() {
        todo!()
    }

    fn stop<L: a653rs::Locked>(
        process_id: ProcessId,
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn delayed_start<L: a653rs::Locked>(
        process_id: ProcessId,
        delay_time: a653rs::bindings::ApexSystemTime,
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn lock_preemption<L: a653rs::Locked>() -> Result<LockLevel, a653rs::bindings::ErrorReturnCode>
    {
        todo!()
    }

    fn unlock_preemption<L: a653rs::Locked>() -> Result<LockLevel, a653rs::bindings::ErrorReturnCode>
    {
        todo!()
    }

    fn get_my_id<L: a653rs::Locked>() -> Result<ProcessId, a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn get_process_id<L: a653rs::Locked>(
        process_name: ProcessName,
    ) -> Result<ProcessId, a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn get_process_status<L: a653rs::Locked>(
        process_id: ProcessId,
    ) -> Result<a653rs::bindings::ApexProcessStatus, a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn initialize_process_core_affinity<L: a653rs::Locked>(
        process_id: ProcessId,
        processor_core_id: a653rs::bindings::ProcessorCoreId,
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn get_my_processor_core_id<L: a653rs::Locked>() -> a653rs::bindings::ProcessorCoreId {
        todo!()
    }

    fn get_my_index<L: a653rs::Locked>() -> Result<ProcessIndex, a653rs::bindings::ErrorReturnCode>
    {
        todo!()
    }
}

impl ApexErrorP4 for DummyHypervisor {
    fn report_application_message<L: a653rs::Locked>(
        message: &[ApexByte],
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        todo!()
    }

    fn raise_application_error<L: a653rs::Locked>(
        error_code: ErrorCode,
        message: &[ApexByte],
    ) -> Result<(), a653rs::bindings::ErrorReturnCode> {
        todo!()
    }
}

#[derive(Debug, Default)]
pub struct DummyScheduler<const SLOTS: usize>;

impl<const SLOTS: usize> Scheduler for DummyScheduler<SLOTS> {
    fn schedule_next(
        &mut self,
        current_time: &core::time::Duration,
    ) -> Option<a653rs_router::prelude::VirtualLinkId> {
        todo!()
    }

    fn reconfigure(
        &mut self,
        vls: &[(a653rs_router::prelude::VirtualLinkId, core::time::Duration)],
    ) -> Result<(), Error> {
        todo!()
    }
}

#[derive(Debug)]
pub struct DummyInterface<const MTU: usize>;

impl<const MTU: usize> CreateNetworkInterfaceId<Self> for DummyInterface<MTU> {
    fn create_network_interface_id(
        cfg: InterfaceConfig,
    ) -> Result<a653rs_router::prelude::NetworkInterfaceId, InterfaceError> {
        todo!()
    }
}

impl<const MTU: usize> PlatformNetworkInterface for DummyInterface<MTU> {
    type Configuration = InterfaceConfig;

    fn platform_interface_send_unchecked(
        id: a653rs_router::prelude::NetworkInterfaceId,
        vl: a653rs_router::prelude::VirtualLinkId,
        buffer: &[u8],
    ) -> Result<usize, InterfaceError> {
        todo!()
    }

    fn platform_interface_receive_unchecked(
        id: a653rs_router::prelude::NetworkInterfaceId,
        buffer: &'_ mut [u8],
    ) -> Result<(a653rs_router::prelude::VirtualLinkId, &'_ [u8]), InterfaceError> {
        todo!()
    }
}
