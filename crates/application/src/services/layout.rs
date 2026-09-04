use crate::error::AppError;
use crate::ports::repositories::{AuditActor, BedRepository, FloorRepository, RoomRepository};
use crate::ports::tenancy::TenantContext;
use pg_domain::{BedId, BranchId, FloorId, RoomId};
use serde::Deserialize;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct OnboardingLayout {
    pub version: u16,
    pub floors: FloorGeneration,
    pub rooms: RoomGeneration,
    pub beds: BedGeneration,
    #[serde(default)]
    pub identity_verification: IdentityVerificationConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IdentityVerificationConfig {
    #[serde(default)]
    pub required_for_tenant_activation: bool,
    #[serde(default = "default_identity_provider")]
    pub provider: String,
    #[serde(default = "enabled")]
    pub allow_manual_review: bool,
}

impl Default for IdentityVerificationConfig {
    fn default() -> Self {
        Self {
            required_for_tenant_activation: false,
            provider: default_identity_provider(),
            allow_manual_review: true,
        }
    }
}

fn default_identity_provider() -> String {
    "digilocker".into()
}

fn enabled() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct FloorGeneration {
    pub start: u16,
    pub count: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoomGeneration {
    pub per_floor: u16,
    #[serde(default = "one")]
    pub start_at: u16,
    #[serde(default)]
    pub overrides: BTreeMap<String, RoomOverride>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BedGeneration {
    pub default_per_room: u16,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RoomOverride {
    pub beds: Option<u16>,
    pub cooling_type: Option<String>,
}

fn one() -> u16 {
    1
}

impl OnboardingLayout {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("version must be 1".into());
        }
        if self.floors.count == 0 || self.floors.count > 100 {
            return Err("floors.count must be between 1 and 100".into());
        }
        if self.rooms.per_floor == 0 || self.rooms.per_floor > 100 {
            return Err("rooms.per_floor must be between 1 and 100".into());
        }
        if self.beds.default_per_room > 26 {
            return Err("beds.default_per_room cannot exceed 26".into());
        }
        let generated: std::collections::BTreeSet<String> = (0..self.floors.count)
            .flat_map(|floor_offset| {
                let floor = self.floors.start + floor_offset;
                (0..self.rooms.per_floor).map(move |room_offset| {
                    format!("{floor}{:02}", self.rooms.start_at + room_offset)
                })
            })
            .collect();
        for (number, override_) in &self.rooms.overrides {
            if !generated.contains(number) {
                return Err(format!(
                    "override references room {number}, which is not generated"
                ));
            }
            if override_.beds.is_some_and(|beds| beds > 26) {
                return Err(format!("room {number} cannot have more than 26 beds"));
            }
            if override_
                .cooling_type
                .as_deref()
                .is_some_and(|value| !matches!(value, "ac" | "non_ac"))
            {
                return Err(format!("room {number} cooling_type must be ac or non_ac"));
            }
        }
        Ok(())
    }
}

pub async fn reconcile_onboarding_layout(
    layout: &OnboardingLayout,
    room_repo: &dyn RoomRepository,
    bed_repo: &dyn BedRepository,
    floor_repo: &dyn FloorRepository,
    ctx: &TenantContext,
    branch_id: BranchId,
    actor: &AuditActor,
) -> Result<(), AppError> {
    layout.validate().map_err(AppError::Validation)?;
    let mut rooms = room_repo.list_by_branch(ctx, branch_id).await?;
    // The layout has always described floors -- it derives room numbers from
    // them ("101" is the first room of floor 1) -- but until floors were a
    // table there was nothing to create. Now a bootstrapped branch arrives
    // fully filed instead of every room landing unassigned.
    let mut floors = floor_repo.list_by_branch(ctx, branch_id).await?;
    for floor_offset in 0..layout.floors.count {
        let floor = layout.floors.start + floor_offset;
        let floor_name = format!("Floor {floor}");
        // Reconciliation re-runs, so an existing floor is reused rather than
        // duplicated -- and the unique index on (branch, name) would refuse it.
        let floor_id = match floors.iter().find(|f| f.name == floor_name) {
            Some(existing) => FloorId::new(existing.id),
            None => {
                let id = FloorId::new(Uuid::new_v4());
                // The bootstrapper creates its rooms itself, further down, so
                // the floor is created bare here.
                floor_repo
                    .create(ctx, id, branch_id, &floor_name, floor as i32, &[], actor)
                    .await?;
                floors.push(crate::ports::repositories::FloorSummary {
                    id: id.as_uuid(),
                    branch_id: branch_id.as_uuid(),
                    name: floor_name.clone(),
                    sort_order: floor as i32,
                    room_count: 0,
                });
                id
            }
        };
        for room_offset in 0..layout.rooms.per_floor {
            let ordinal = layout.rooms.start_at + room_offset;
            let number = format!("{floor}{ordinal:02}");
            let override_ = layout.rooms.overrides.get(&number);
            let cooling = override_
                .and_then(|x| x.cooling_type.as_deref())
                .unwrap_or("non_ac");
            if !matches!(cooling, "ac" | "non_ac") {
                return Err(AppError::Validation(format!(
                    "invalid cooling_type for room {number}"
                )));
            }
            let room_id = match rooms.iter().find(|r| r.number == number) {
                Some(room) => RoomId::new(room.id),
                None => {
                    let id = RoomId::new(Uuid::new_v4());
                    room_repo
                        .create(
                            ctx,
                            id,
                            branch_id,
                            &number,
                            cooling,
                            Some(floor_id),
                            0,
                            actor,
                        )
                        .await?;
                    id
                }
            };
            let existing_beds = bed_repo.list_by_room(ctx, room_id).await?;
            let bed_count = override_
                .and_then(|x| x.beds)
                .unwrap_or(layout.beds.default_per_room);
            for index in 0..bed_count {
                let label = char::from_u32(u32::from(b'A') + u32::from(index))
                    .ok_or_else(|| AppError::Validation("beds per room cannot exceed 26".into()))?
                    .to_string();
                if !existing_beds.iter().any(|bed| bed.label == label) {
                    bed_repo
                        .create(ctx, BedId::new(Uuid::new_v4()), room_id, &label, actor)
                        .await?;
                }
            }
            if !rooms.iter().any(|r| r.number == number) {
                rooms.push(crate::ports::repositories::RoomSummary {
                    id: room_id.as_uuid(),
                    branch_id: branch_id.as_uuid(),
                    number,
                    cooling_type: cooling.into(),
                    floor_id: Some(floor_id.as_uuid()),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> OnboardingLayout {
        OnboardingLayout {
            version: 1,
            floors: FloorGeneration { start: 1, count: 6 },
            rooms: RoomGeneration {
                per_floor: 3,
                start_at: 1,
                overrides: BTreeMap::from([(
                    "101".into(),
                    RoomOverride {
                        beds: Some(2),
                        cooling_type: Some("ac".into()),
                    },
                )]),
            },
            beds: BedGeneration {
                default_per_room: 3,
            },
            identity_verification: IdentityVerificationConfig::default(),
        }
    }

    #[test]
    fn valid_generated_layout_is_accepted() {
        assert!(sample().validate().is_ok());
    }

    #[test]
    fn override_for_non_generated_room_is_rejected() {
        let mut layout = sample();
        layout
            .rooms
            .overrides
            .insert("999".into(), RoomOverride::default());
        assert!(layout.validate().is_err());
    }

    #[test]
    fn more_than_twenty_six_lettered_beds_is_rejected() {
        let mut layout = sample();
        layout.beds.default_per_room = 27;
        assert!(layout.validate().is_err());
    }
}
