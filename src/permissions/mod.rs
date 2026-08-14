use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPolicy {
    pub admin_role_ids: HashSet<u64>,
    pub allowed_channel_ids: HashSet<u64>,
    pub blocked_user_ids: HashSet<u64>,
    pub blocked_track_ids: HashSet<String>,
}
impl Default for PermissionPolicy {
    fn default() -> Self {
        Self {
            admin_role_ids: HashSet::new(),
            allowed_channel_ids: HashSet::new(),
            blocked_user_ids: HashSet::new(),
            blocked_track_ids: HashSet::new(),
        }
    }
}
impl PermissionPolicy {
    pub fn can_use(&self, user_id: u64, channel_id: u64, roles: &[u64]) -> bool {
        !self.blocked_user_ids.contains(&user_id)
            && (self.allowed_channel_ids.is_empty()
                || self.allowed_channel_ids.contains(&channel_id))
            && (self.admin_role_ids.is_empty()
                || roles.iter().any(|role| self.admin_role_ids.contains(role)))
    }
    pub fn can_play_track(&self, user_id: u64, track_id: &str) -> bool {
        !self.blocked_user_ids.contains(&user_id) && !self.blocked_track_ids.contains(track_id)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn policy_checks() {
        let mut p = PermissionPolicy::default();
        p.blocked_user_ids.insert(2);
        assert!(!p.can_use(2, 1, &[]));
        assert!(p.can_use(1, 1, &[]));
    }
}
