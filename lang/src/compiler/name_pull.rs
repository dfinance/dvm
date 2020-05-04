
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
struct Container<'a>(&'a str);

impl Container<'static> {
    fn as_static(&self) -> &'static str {
        self.0
    }
}

unsafe fn extend_lifetime<'b>(r: Container<'b>) -> Container<'static> {
    std::mem::transmute::<Container<'b>, Container<'static>>(r)
}

unsafe fn shorten_invariant_lifetime<'b, 'c>(
    r: &'b mut Container<'static>,
) -> &'b mut Container<'c> {
    std::mem::transmute::<&'b mut Container<'static>, &'b mut Container<'c>>(r)
}

// Static string auto release pool.
pub struct StaticHolder {
    pull: Vec<Container<'static>>,
}

impl StaticHolder {
    /// Create new name pull.
    pub fn new() -> Self {
        StaticHolder {
            pull: Default::default(),
        }
    }

    // Put string to pull.
    pub fn pull(&mut self, val: &str) -> &'static str {
        let container = Container(val);
        let static_container: Container<'static> = unsafe { extend_lifetime(container) };
        let static_name = static_container.as_static();
        self.pull.push(static_container);
        static_name
    }
}

impl Drop for StaticHolder {
    fn drop(&mut self) {
        for container in &mut self.pull {
            unsafe { shorten_invariant_lifetime(container) };
        }
    }
}
