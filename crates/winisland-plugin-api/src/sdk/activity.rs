use crate::{ActivityApiV2, ActivitySpecV2, IFACE_ACTIVITY, ResourceId};

use super::{Error, Host, success};

#[derive(Clone)]
pub struct Activities(Host);

pub struct Activity {
    host: Host,
    id: ResourceId,
}

impl Host {
    pub fn activities(&self) -> Result<Activities, Error> {
        self.query::<ActivityApiV2>(IFACE_ACTIVITY)?;
        Ok(Activities(self.clone()))
    }
}

impl Activities {
    pub fn create(&self, spec: ActivitySpecV2) -> Result<Activity, Error> {
        let table = self.0.query::<ActivityApiV2>(IFACE_ACTIVITY)?;
        let create = table
            .create
            .ok_or(Error::MissingFunction("activity.create"))?;
        let mut id = ResourceId::INVALID;
        // SAFETY: The specification and resource output remain live for this synchronous call.
        success(unsafe { create(table.prefix.context, self.0.token, &spec, &mut id) })?;
        if id == ResourceId::INVALID {
            return Err(Error::InvalidHost);
        }
        Ok(Activity {
            host: self.0.clone(),
            id,
        })
    }
}

impl Activity {
    pub fn id(&self) -> ResourceId {
        self.id
    }

    pub fn update(&self, spec: ActivitySpecV2) -> Result<(), Error> {
        let table = self.host.query::<ActivityApiV2>(IFACE_ACTIVITY)?;
        let update = table
            .update
            .ok_or(Error::MissingFunction("activity.update"))?;
        // SAFETY: The host validates the owned resource and the live specification before use.
        success(unsafe { update(table.prefix.context, self.host.token, self.id, &spec) })
    }
}

impl Drop for Activity {
    fn drop(&mut self) {
        if let Ok(table) = self.host.query::<ActivityApiV2>(IFACE_ACTIVITY)
            && let Some(release) = table.release
        {
            // SAFETY: The host validates the owned resource before releasing it.
            let _ = unsafe { release(table.prefix.context, self.host.token, self.id) };
        }
    }
}
