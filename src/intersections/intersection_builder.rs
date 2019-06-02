use std::convert::TryFrom;
use std::sync::{Arc, RwLock};

use crossbeam_channel::Sender;
use failure::Fail;

use crate::config::blocks::Blocks;
use crate::config::definitions::{Component as ConfigComponent, Definitions, Group as ConfigGroup};
use crate::intersections::actuator::{Actuator, ArcActuator};
use crate::intersections::component::{ComponentId, ComponentKind};
use crate::intersections::deck::DeckState;
use crate::intersections::gate::GateState;
use crate::intersections::group::{Group, GroupId, GroupKind};
use crate::intersections::intersection::Notification;
use crate::intersections::intersection::{ArcIntersection, Intersection};
use crate::intersections::light::LightState;
use crate::intersections::sensor::{Sensor, SensorState};

#[derive(Debug, Fail)]
#[fail(display = "Intersection build error")]
pub struct IntersectionBuildError;

pub struct IntersectionsBuilder<'a> {
    defs: Option<&'a Definitions>,
    blocks: Option<&'a Blocks>,
    notification_sender: Sender<Notification>,
}

impl<'a> IntersectionsBuilder<'a> {
    pub fn new(notification_sender: Sender<Notification>) -> IntersectionsBuilder<'a> {
        Self {
            defs: None,
            blocks: None,
            notification_sender,
        }
    }

    pub fn with_defs(mut self, defs: &'a Definitions) -> Self {
        self.defs = Some(defs);
        self
    }

    pub fn with_blocks(mut self, blocks: &'a Blocks) -> Self {
        self.blocks = Some(blocks);
        self
    }

    pub fn finish(&self) -> Result<ArcIntersection, failure::Error> {
        let defs = match self.defs {
            Some(defs) => defs,
            None => {
                return Err(IntersectionBuildError.into());
            }
        };

        let intersection = Arc::new(RwLock::new(Box::new(Intersection::new(
            None,
            self.notification_sender.clone(),
        ))));

        self.build_groups(&defs.groups, Arc::clone(&intersection))?;

        Ok(self.fill_concurrences(self.fill_blocks(intersection)?)?)
    }

    pub fn build_groups(
        &self,
        conf_groups: &[ConfigGroup],
        intersection: ArcIntersection,
    ) -> Result<(), failure::Error> {
        for conf_group in conf_groups {
            let id = GroupId {
                id: conf_group.id,
                kind: GroupKind::try_from(&conf_group.kind[..])?,
            };

            let group = Arc::new(RwLock::new(Box::new(Group::new(
                Arc::clone(&intersection),
                id,
                match conf_group.can_be_blocked {
                    Some(can_be_blocked) => can_be_blocked,
                    None => false,
                },
            ))));

            if let Some(conf_cmpts) = &conf_group.components {
                self.build_components(&conf_cmpts, Arc::clone(&group))?;
            }

            intersection
                .write()
                .unwrap()
                .groups
                .insert(id, Arc::clone(&group));
        }

        Ok(())
    }

    pub fn build_components(
        &self,
        conf_cmpts: &[ConfigComponent],
        group: Arc<RwLock<Box<Group>>>,
    ) -> Result<(), failure::Error> {
        for conf_compt in conf_cmpts {
            let id = ComponentId {
                id: conf_compt.id,
                kind: ComponentKind::try_from(&conf_compt.kind[..])?,
            };

            match id.kind {
                ComponentKind::Sensor => {
                    let component = Arc::new(RwLock::new(Box::new(Sensor::new(
                        Arc::clone(&group),
                        id,
                        match conf_compt.initial_state {
                            Some(state) => SensorState::try_from(state)?,
                            None => SensorState::default(),
                        },
                        match conf_compt.distance {
                            Some(distance) => distance,
                            None => 0,
                        },
                    ))));

                    group.write().unwrap().sensors.insert(id, component);
                }
                ComponentKind::Light => {
                    let component: ArcActuator<LightState> =
                        Arc::new(RwLock::new(Box::new(Actuator::new(
                            Arc::clone(&group),
                            id,
                            match conf_compt.initial_state {
                                Some(state) => LightState::try_from(state)?,
                                None => LightState::default(),
                            },
                        ))));

                    group.write().unwrap().lights.insert(id, component);
                }
                ComponentKind::Gate => {
                    let component: ArcActuator<GateState> =
                        Arc::new(RwLock::new(Box::new(Actuator::new(
                            Arc::clone(&group),
                            id,
                            match conf_compt.initial_state {
                                Some(state) => GateState::try_from(state)?,
                                None => GateState::default(),
                            },
                        ))));

                    group.write().unwrap().gates.insert(id, component);
                }
                ComponentKind::Deck => {
                    let component: ArcActuator<DeckState> =
                        Arc::new(RwLock::new(Box::new(Actuator::new(
                            Arc::clone(&group),
                            id,
                            match conf_compt.initial_state {
                                Some(state) => DeckState::try_from(state)?,
                                None => DeckState::default(),
                            },
                        ))));

                    group.write().unwrap().decks.insert(id, component);
                }
            };
        }

        Ok(())
    }

    fn fill_blocks(
        &self,
        intersection: ArcIntersection,
    ) -> Result<ArcIntersection, failure::Error> {
        if let None = self.blocks {
            return Ok(intersection);
        }

        for blocked_group in &self.blocks.unwrap().groups {
            let actual_group = intersection.read().unwrap().find_group(GroupId {
                id: blocked_group.id,
                kind: GroupKind::try_from(&blocked_group.kind[..])?,
            });

            if actual_group.is_none() {
                continue;
            }

            let actual_group = actual_group.unwrap();

            for block in &blocked_group.blocks {
                let found_group = intersection
                    .read()
                    .unwrap()
                    .find_group(GroupId {
                        kind: GroupKind::try_from(&block.kind[..])?,
                        id: block.id,
                    })
                    .unwrap();

                actual_group
                    .write()
                    .unwrap()
                    .push_block(Arc::clone(&found_group));
            }
        }

        Ok(intersection)
    }

    fn fill_concurrences(
        &self,
        intersection: ArcIntersection,
    ) -> Result<ArcIntersection, failure::Error> {
        for outer_group in intersection.read().unwrap().groups() {
            for inner_group in intersection.read().unwrap().groups() {
                if !outer_group
                    .read()
                    .unwrap()
                    .blocks_group(Arc::clone(&inner_group))
                {
                    outer_group
                        .write()
                        .unwrap()
                        .push_concurrent(Arc::clone(&inner_group));
                }
            }
        }

        Ok(intersection)
    }
}
