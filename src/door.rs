use crate::actor::ActorContainer;
use rg3d::core::color::Color;
use rg3d::{
    core::{
        algebra::{Isometry3, Translation3, Vector3},
        pool::{Handle, Pool},
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::{graph::Graph, node::Node, Scene},
};

#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(u32)]
pub enum DoorState {
    Opened = 0,
    Opening = 1,
    Closed = 2,
    Closing = 3,
    Locked = 4,
    Broken = 5,
}

impl Default for DoorState {
    fn default() -> Self {
        Self::Closed
    }
}

impl DoorState {
    pub fn id(self) -> u32 {
        self as u32
    }

    pub fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Opened),
            1 => Ok(Self::Opening),
            2 => Ok(Self::Closed),
            3 => Ok(Self::Closing),
            4 => Ok(Self::Locked),
            5 => Ok(Self::Broken),
            _ => Err(format!("Invalid door state id {}!", id)),
        }
    }
}

impl Visit for DoorState {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut id = self.id();
        id.visit(name, visitor)?;
        if visitor.is_reading() {
            *self = Self::from_id(id)?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct Door {
    node: Handle<Node>,
    lights: Vec<Handle<Node>>,
    state: DoorState,
    offset: f32,
    initial_position: Vector3<f32>,
}

impl Visit for Door {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.node.visit("Node", visitor)?;
        self.lights.visit("Lights", visitor)?;
        self.state.visit("State", visitor)?;
        self.offset.visit("Offset", visitor)?;

        visitor.leave_region()
    }
}

impl Door {
    pub fn new(node: Handle<Node>, graph: &Graph, state: DoorState) -> Self {
        Self {
            node,
            lights: graph
                .traverse_handle_iter(node)
                .filter(|&handle| graph[handle].is_light())
                .collect(),
            state,
            offset: 0.0,
            initial_position: graph[node].global_position(),
        }
    }

    pub fn resolve(&mut self, scene: &Scene) {
        self.initial_position = scene.graph[self.node].global_position();
    }

    fn set_lights_color(&self, graph: &mut Graph, color: Color) {
        for &light in self.lights.iter() {
            graph[light].as_light_mut().set_color(color);
        }
    }

    fn set_lights_enabled(&self, graph: &mut Graph, enabled: bool) {
        for &light in self.lights.iter() {
            graph[light].set_visibility(enabled);
        }
    }
}

#[derive(Default)]
pub struct DoorContainer {
    doors: Pool<Door>,
}

impl DoorContainer {
    pub fn new() -> Self {
        Self {
            doors: Default::default(),
        }
    }

    pub fn add(&mut self, door: Door) -> Handle<Door> {
        self.doors.spawn(door)
    }

    pub fn update(&mut self, actors: &ActorContainer, scene: &mut Scene, dt: f32) {
        for door in self.doors.iter_mut() {
            let node = &scene.graph[door.node];
            let door_side = node.look_vector();

            let need_to_open = actors.iter().any(|a| {
                let actor_position = a.position(&scene.graph);
                // TODO: Replace with triggers.
                actor_position.metric_distance(&door.initial_position) < 1.25
            });

            if need_to_open {
                if door.state == DoorState::Closed {
                    door.state = DoorState::Opening;
                }
            } else if door.state == DoorState::Opened {
                door.state = DoorState::Closing;
            }

            match door.state {
                DoorState::Opening => {
                    if door.offset < 0.75 {
                        door.offset += 1.0 * dt;
                        if door.offset >= 0.75 {
                            door.state = DoorState::Opened;
                            door.offset = 0.75;
                        }
                    }

                    door.set_lights_enabled(&mut scene.graph, false);
                }
                DoorState::Closing => {
                    if door.offset > 0.0 {
                        door.offset -= 1.0 * dt;
                        if door.offset <= 0.0 {
                            door.state = DoorState::Closed;
                            door.offset = 0.0;
                        }
                    }

                    door.set_lights_enabled(&mut scene.graph, false);
                }
                DoorState::Closed => {
                    door.set_lights_enabled(&mut scene.graph, true);
                    door.set_lights_color(&mut scene.graph, Color::opaque(0, 255, 0));
                }
                DoorState::Locked => {
                    door.set_lights_enabled(&mut scene.graph, true);
                    door.set_lights_color(&mut scene.graph, Color::opaque(255, 0, 0));
                }
                DoorState::Broken | DoorState::Opened => {
                    door.set_lights_enabled(&mut scene.graph, false);
                }
            };

            if let Some(body) = scene.physics_binder.body_of(door.node) {
                let body = scene.physics.bodies.get_mut(body.into()).unwrap();
                body.set_position(
                    Isometry3 {
                        translation: Translation3 {
                            vector: door.initial_position
                                + door_side
                                    .try_normalize(std::f32::EPSILON)
                                    .unwrap_or_default()
                                    .scale(door.offset),
                        },
                        rotation: body.position().rotation,
                    },
                    true,
                );
            }
        }
    }

    pub fn resolve(&mut self, scene: &Scene) {
        for door in self.doors.iter_mut() {
            door.resolve(scene)
        }
    }
}

impl Visit for DoorContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.doors.visit("Doors", visitor)?;

        visitor.leave_region()
    }
}
