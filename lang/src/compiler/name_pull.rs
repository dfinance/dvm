unsafe fn extend_lifetime(r: &str) -> &'static str {
    std::mem::transmute::<&str, &'static str>(r)
}

// Static string auto release pool.
#[derive(Default)]
pub struct StrTable {
    pull: Vec<String>,
}

impl StrTable {
    /// Create new name pull.
    pub fn new() -> Self {
        StrTable {
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

// #[cfg(test)]
// mod tests {
//     use crate::compiler::name_pull::StaticHolder;
//     use rand::{thread_rng, Rng};
//     use std::time::Duration;
//     use std::thread;
//
//     fn test_d() {
//         loop {
//             {
//                 let mut pull = StaticHolder::new();
//                 let mut rng = thread_rng();
//                 for _ in 0..100000 {
//                     let name: [f32; 32] = rng.gen();
//                     pull.pull(format!("{:?}", name));
//                 }
//                 //thread::sleep(Duration::from_secs(1));
//             }
//            // thread::sleep(Duration::from_secs(10));
//         }
//     }
// }
