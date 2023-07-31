use crate::{
    config::Config,
    error::Error,
    router::{Router, RouterInput, RouterOutput},
    scheduler::Scheduler,
};
use core::fmt::Debug;
use heapless::{LinearMap, String, Vec};
use log::{debug, warn};

#[cfg(feature = "serde")]
use crate::types::VirtualLinkId;

/// Collection of inputs and outputs.
///
/// Use this to pass resources like ports into the network partition.
#[derive(Clone, Default)]
pub struct Resources<'a, const I: usize, const O: usize> {
    inputs: LinearMap<String<20>, &'a dyn RouterInput, I>,
    outputs: LinearMap<String<20>, &'a dyn RouterOutput, O>,
}

impl<'a, const I: usize, const O: usize> Debug for Resources<'a, I, O> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let inputs: Vec<_, I> = self.inputs.keys().collect();
        let outputs: Vec<_, O> = self.outputs.keys().collect();
        f.write_fmt(format_args!(
            "Resources: inputs: {:?}, outputs: {:?}",
            inputs, outputs,
        ))
    }
}

impl<'a, const I: usize, const O: usize> Resources<'a, I, O> {
    /// Retrieves an input.
    pub fn get_input(&self, name: &str) -> Option<&'a dyn RouterInput> {
        self.inputs.get(&String::from(name)).cloned()
    }

    /// Retrieves an output.
    pub fn get_output(&self, name: &str) -> Option<&'a dyn RouterOutput> {
        self.outputs.get(&String::from(name)).cloned()
    }

    /// Creates a new empty resource collection.
    pub fn new() -> Self {
        Self {
            inputs: Default::default(),
            outputs: Default::default(),
        }
    }

    /// Insert a new input.
    pub fn insert_input<'c>(&mut self, name: &str, value: &'c dyn RouterInput) -> Result<(), Error>
    where
        'c: 'a,
    {
        _ = self
            .inputs
            .insert(String::from(name), value)
            .or(Err(Error::InvalidConfig))?;
        Ok(())
    }

    /// Insert a new output.
    pub fn insert_output<'c>(
        &mut self,
        name: &str,
        value: &'c dyn RouterOutput,
    ) -> Result<(), Error>
    where
        'c: 'a,
    {
        _ = self
            .outputs
            .insert(String::from(name), value)
            .or(Err(Error::InvalidConfig))?;
        Ok(())
    }

    /// Creates a new resources of a different size from the given resources.
    pub fn grow<const I2: usize, const O2: usize>(self) -> Resources<'a, I2, O2> {
        Resources::<'a, I2, O2> {
            inputs: LinearMap::from_iter(self.inputs.into_iter().map(|(k, v)| (k.clone(), *v))),
            outputs: LinearMap::from_iter(self.outputs.into_iter().map(|(k, v)| (k.clone(), *v))),
        }
    }
}

/// Configurator
#[derive(Debug, Clone)]
pub struct Configurator;

impl Configurator {
    /// Obtains a router for the next configuration and updates the scheduler.
    pub fn reconfigure<'a, const I: usize, const O: usize>(
        resources: &Resources<'a, I, O>,
        scheduler: &mut dyn Scheduler,
        cfg: &Config<I, O>,
    ) -> Result<Router<'a, I, O>, Error> {
        debug!("Have resources {resources:?}");
        let mut b = &mut crate::router::builder();
        if cfg.vls.is_empty() {
            return b.build();
        }
        let slots: Vec<_, I> = cfg.vls.into_iter().map(|(v, c)| (*v, c.period)).collect();
        scheduler.reconfigure(slots.as_slice())?;
        for (v, cfg) in cfg.vls.into_iter() {
            debug!("VL {v} got config {cfg:?}");
            let input = cfg.src.as_str();
            let inp = resources.get_input(input).ok_or_else(|| {
                warn!("Unknown input: {input}");
                Error::InvalidConfig
            })?;
            let outs: Result<Vec<_, O>, Error> = cfg
                .dsts
                .iter()
                .map(|d| {
                    let output = d.as_str();
                    resources.get_output(output).ok_or_else(|| {
                        warn!("Unknown output {output}");
                        Error::InvalidConfig
                    })
                })
                .collect();
            let outs = outs?;
            b = b.route(v, inp, &outs).or(Err(Error::InvalidConfig))?;
        }
        b.build()
    }

    /// Fetches the configuration from the hypervisor.
    #[cfg(feature = "serde")]
    pub fn fetch_config<const I: usize, const O: usize>(
        config_port: &dyn RouterInput,
    ) -> Result<Config<I, O>, Error> {
        let buf = &mut [0u8; 10000];
        let (_vl, buf) = config_port.receive(&VirtualLinkId::from(0u16), buf)?;
        postcard::from_bytes::<Config<I, O>>(buf).or(Err(Error::InvalidConfig))
    }
}
