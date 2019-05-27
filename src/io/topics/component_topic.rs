use std::convert::TryFrom;
use std::fmt::{Display, Formatter};

use failure::Fail;

use crate::intersections::component::{ComponentId, ComponentKind, ComponentUid};
use crate::intersections::group::{GroupId, GroupKind};
use crate::io::topics::{NoTeamIdSet, Topic};

#[derive(Debug, Fail)]
pub enum ComponentTopicBuildError {
    #[fail(display = "Component topic could not be built, invalid amount of parts.")]
    InvalidAmountOfTopicParts,
}

#[derive(Clone, PartialEq, Hash, Eq)]
pub struct ComponentTopic {
    pub team_id: Option<i32>,
    pub uid: ComponentUid,
}

impl Topic for ComponentTopic {
    fn team_id(&self) -> Result<i32, failure::Error> {
        match self.team_id {
            Some(team_id) => Ok(team_id),
            None => Err(NoTeamIdSet.into()),
        }
    }

    fn set_team_id(&mut self, team_id: i32) {
        self.team_id = Some(team_id)
    }
}

impl TryFrom<&str> for ComponentTopic {
    type Error = failure::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = value.split('/').collect();

        if parts.len() != 5 {
            return Err(ComponentTopicBuildError::InvalidAmountOfTopicParts.into());
        }

        Ok(Self {
            team_id: Some(parts[0].parse::<i32>()?),
            uid: ComponentUid {
                group_id: GroupId {
                    kind: GroupKind::try_from(parts[1])?,
                    id: parts[2].parse::<i32>()?,
                },
                component_id: ComponentId {
                    kind: ComponentKind::try_from(parts[3])?,
                    id: parts[4].parse::<i32>()?,
                },
            },
        })
    }
}

impl From<ComponentUid> for ComponentTopic {
    fn from(uid: ComponentUid) -> Self {
        Self { team_id: None, uid }
    }
}

impl Display for ComponentTopic {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let team_id = match self.team_id {
            Some(team_id) => format!("{}", team_id),
            None => String::from("None"),
        };

        write!(f, "{}/{}", team_id, self.uid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_topic() {
        let raw_topic = "18/motor_vehicle/3/light/1";
        let topic = ComponentTopic::try_from(raw_topic);

        assert!(topic.is_ok());

        let topic = topic.unwrap();

        assert_eq!(topic.team_id, Some(18));
        assert_eq!(topic.uid.group_id.kind, GroupKind::MotorVehicle);
        assert_eq!(topic.uid.group_id.id, 3);
        assert_eq!(topic.uid.component_id.kind, ComponentKind::Light);
        assert_eq!(topic.uid.component_id.id, 1);
    }

    #[test]
    fn test_invalid_kinds() {
        let raw_topic = "18/car/3/lamp/1";
        let topic = ComponentKind::try_from(raw_topic);

        assert!(topic.is_err());
    }
}
