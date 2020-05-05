unsafe fn extend_lifetime(r: &str) -> &'static str {
    std::mem::transmute::<&str, &'static str>(r)
}

// Static string auto release pool.
#[derive(Default)]
pub struct StaticHolder {
    pull: Vec<String>,
}

impl StaticHolder {
    /// Create new name pull.
    pub fn new() -> Self {
        StaticHolder {
            pull: Default::default(),
        }
    }

    // Put string to pull.
    pub fn pull(&mut self, val: String) -> &'static str {
        let static_val = unsafe { extend_lifetime(&val) };
        self.pull.push(val);
        static_val
    }
}

impl Drop for StaticHolder {
    fn drop(&mut self) {
        for container in &mut self.pull {
            unsafe { shorten_invariant_lifetime(container) };
        }
    }
}
