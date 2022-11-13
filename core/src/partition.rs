use apex_rs::prelude::{ApexPartitionP4, Name, Partition};

/// A partition that defines its own name
pub trait PartitionName<P>: Partition<P>
where
    P: ApexPartitionP4,
{
    /// The name of the partition.
    fn name(&self) -> Name;
}
