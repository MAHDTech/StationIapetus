use crate::{
    player::{make_hit_reaction_state, HitReactionStateDefinition},
    utils::{
        create_play_animation_state, fetch_animation_container_mut, fetch_animation_container_ref,
    },
};
use fyrox::{
    animation::{
        machine::{
            node::blend::{BlendPose, IndexedBlendInput},
            LayerMask, Machine, MachineLayer, Parameter, PoseNode, PoseWeight, State, Transition,
        },
        value::{TrackValue, ValueBinding},
        Animation, AnimationSignal,
    },
    core::{
        pool::Handle,
        uuid::{uuid, Uuid},
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::resource_manager::ResourceManager,
    resource::model::Model,
    scene::{node::Node, Scene},
};

pub struct IdleStateDefinition {
    state: Handle<State>,
}

impl IdleStateDefinition {
    pub fn new(
        layer: &mut MachineLayer,
        scene: &mut Scene,
        model: Handle<Node>,
        idle_animation_resource: Model,
        idle_pistol_animation_resource: Model,
        index_parameter: String,
        animation_player: Handle<Node>,
    ) -> Self {
        let idle_animation = *idle_animation_resource
            .retarget_animations(model, &mut scene.graph)
            .get(0)
            .unwrap();
        let idle_animation_node = layer.add_node(PoseNode::make_play_animation(idle_animation));

        let idle_pistol_animation = *idle_pistol_animation_resource
            .retarget_animations(model, &mut scene.graph)
            .get(0)
            .unwrap();

        fetch_animation_container_mut(&mut scene.graph, animation_player)[idle_pistol_animation]
            .set_speed(0.25);

        let idle_pistol_animation_node =
            layer.add_node(PoseNode::make_play_animation(idle_pistol_animation));

        let idle_node = layer.add_node(PoseNode::make_blend_animations_by_index(
            index_parameter,
            vec![
                IndexedBlendInput {
                    blend_time: 0.1,
                    pose_source: idle_animation_node,
                },
                IndexedBlendInput {
                    blend_time: 0.1,
                    pose_source: idle_pistol_animation_node,
                },
            ],
        ));

        Self {
            state: layer.add_state(State::new("Idle", idle_node)),
        }
    }
}

struct WalkStateDefinition {
    state: Handle<State>,
    walk_animation: Handle<Animation>,
    run_animation: Handle<Animation>,
}

impl WalkStateDefinition {
    fn new(
        layer: &mut MachineLayer,
        scene: &mut Scene,
        model: Handle<Node>,
        walk_animation_resource: Model,
        walk_pistol_animation_resource: Model,
        run_animation_resource: Model,
        run_pistol_animation_resource: Model,
        index: String,
    ) -> Self {
        let walk_animation = *walk_animation_resource
            .retarget_animations(model, &mut scene.graph)
            .get(0)
            .unwrap();
        let walk_animation_node = layer.add_node(PoseNode::make_play_animation(walk_animation));

        let walk_pistol_animation = *walk_pistol_animation_resource
            .retarget_animations(model, &mut scene.graph)
            .get(0)
            .unwrap();
        let walk_pistol_animation_node =
            layer.add_node(PoseNode::make_play_animation(walk_pistol_animation));

        let run_animation = *run_animation_resource
            .retarget_animations(model, &mut scene.graph)
            .get(0)
            .unwrap();
        let run_animation_node = layer.add_node(PoseNode::make_play_animation(run_animation));

        let run_pistol_animation = *run_pistol_animation_resource
            .retarget_animations(model, &mut scene.graph)
            .get(0)
            .unwrap();
        let run_pistol_animation_node =
            layer.add_node(PoseNode::make_play_animation(run_pistol_animation));

        let walk_node = layer.add_node(PoseNode::make_blend_animations_by_index(
            index,
            vec![
                IndexedBlendInput {
                    blend_time: 0.5,
                    pose_source: walk_animation_node,
                },
                IndexedBlendInput {
                    blend_time: 0.5,
                    pose_source: walk_pistol_animation_node,
                },
                IndexedBlendInput {
                    blend_time: 0.5,
                    pose_source: run_animation_node,
                },
                IndexedBlendInput {
                    blend_time: 0.5,
                    pose_source: run_pistol_animation_node,
                },
            ],
        ));

        Self {
            state: layer.add_state(State::new("Walk", walk_node)),
            walk_animation,
            run_animation,
        }
    }
}

#[derive(Default, Visit, Debug)]
pub struct UpperBodyMachine {
    pub machine: Machine,
    pub aim_state: Handle<State>,
    pub toss_grenade_state: Handle<State>,
    pub put_back_state: Handle<State>,
    pub jump_animation: Handle<Animation>,
    pub walk_animation: Handle<Animation>,
    pub run_animation: Handle<Animation>,
    pub land_animation: Handle<Animation>,
    pub toss_grenade_animation: Handle<Animation>,
    pub put_back_animation: Handle<Animation>,
    pub grab_animation: Handle<Animation>,
    pub dying_animation: Handle<Animation>,
    pub hit_reaction_pistol_animation: Handle<Animation>,
    pub hit_reaction_rifle_animation: Handle<Animation>,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum CombatWeaponKind {
    Pistol,
    Rifle,
}

pub struct UpperBodyMachineInput {
    pub is_walking: bool,
    pub is_jumping: bool,
    pub run_factor: f32,
    pub has_ground_contact: bool,
    pub is_aiming: bool,
    pub toss_grenade: bool,
    pub weapon_kind: CombatWeaponKind,
    pub change_weapon: bool,
    pub is_dead: bool,
    pub should_be_stunned: bool,
}

impl UpperBodyMachine {
    const WALK_TO_AIM: &'static str = "WalkToAim";
    const IDLE_TO_AIM: &'static str = "IdleToAim";
    const AIM_TO_IDLE: &'static str = "AimToIdle";
    const AIM_TO_WALK: &'static str = "AimToWalk";

    const WALK_TO_IDLE: &'static str = "WalkToIdle";
    const WALK_TO_JUMP: &'static str = "WalkToJump";
    const IDLE_TO_WALK: &'static str = "IdleToWalk";
    const IDLE_TO_JUMP: &'static str = "IdleToJump";
    const JUMP_TO_FALL: &'static str = "JumpToFall";
    const WALK_TO_FALL: &'static str = "WalkToFall";
    const IDLE_TO_FALL: &'static str = "IdleToFall";
    const FALL_TO_LAND: &'static str = "FallToLand";
    const LAND_TO_IDLE: &'static str = "LandToIdle";
    const TOSS_GRENADE_TO_AIM: &'static str = "TossGrenadeToAim";
    const AIM_TO_TOSS_GRENADE: &'static str = "AimToTossGrenade";

    const AIM_TO_PUT_BACK: &'static str = "AimToPutBack";
    const WALK_TO_PUT_BACK: &'static str = "WalkToPutBack";
    const IDLE_TO_PUT_BACK: &'static str = "IdleToPutBack";

    const PUT_BACK_TO_IDLE: &'static str = "PutBackToIdle";
    const PUT_BACK_TO_WALK: &'static str = "PutBackToWalk";

    const PUT_BACK_TO_GRAB: &'static str = "PutBackToGrab";
    const GRAB_TO_IDLE: &'static str = "GrabToIdle";
    const GRAB_TO_WALK: &'static str = "GrabToWalk";
    const GRAB_TO_AIM: &'static str = "GrabToAim";

    const LAND_TO_DYING: &'static str = "LandToDying";
    const FALL_TO_DYING: &'static str = "FallToDying";
    const IDLE_TO_DYING: &'static str = "IdleToDying";
    const WALK_TO_DYING: &'static str = "WalkToDying";
    const JUMP_TO_DYING: &'static str = "JumpToDying";
    const AIM_TO_DYING: &'static str = "AimToDying";
    const TOSS_GRENADE_TO_DYING: &'static str = "TossGrenadeToDying";
    const GRAB_TO_DYING: &'static str = "GrabToDying";
    const PUT_BACK_TO_DYING: &'static str = "PutBackToDying";

    const RIFLE_AIM_FACTOR: &'static str = "RifleAimFactor";
    const PISTOL_AIM_FACTOR: &'static str = "PistolAimFactor";

    const IDLE_TO_HIT_REACTION: &'static str = "IdleToHitReaction";
    const WALK_TO_HIT_REACTION: &'static str = "WalkToHitReaction";
    const AIM_TO_HIT_REACTION: &'static str = "AimToHitReaction";
    const HIT_REACTION_TO_IDLE: &'static str = "HitReactionToIdle";
    const HIT_REACTION_TO_WALK: &'static str = "HitReactionToWalk";
    const HIT_REACTION_TO_DYING: &'static str = "HitReactionToDying";
    const HIT_REACTION_TO_AIM: &'static str = "HitReactionToAim";

    const HIT_REACTION_WEAPON_KIND: &'static str = "HitReactionWeaponKind";
    const IDLE_STATE_WEAPON_KIND: &'static str = "IdleStateWeaponKind";
    const WALK_STATE_WEAPON_KIND: &'static str = "IdleStateWeaponKind";

    pub const GRAB_WEAPON_SIGNAL: Uuid = uuid!("4b80a4ac-b782-44c6-a6d6-cdead72f5369");
    pub const PUT_BACK_WEAPON_END_SIGNAL: Uuid = uuid!("a923cabd-da6a-43ca-85cc-861370b1669a");
    pub const TOSS_GRENADE_SIGNAL: Uuid = uuid!("ce07b80a-e099-4cc5-8361-43d6631f431c");

    pub async fn new(
        scene: &mut Scene,
        model: Handle<Node>,
        resource_manager: ResourceManager,
        animation_player: Handle<Node>,
    ) -> Self {
        let mut machine = Machine::new();

        let root_layer = machine.layers_mut().first_mut().unwrap();

        let mut layer_mask = LayerMask::default();
        for leg_name in &["mixamorig:LeftUpLeg", "mixamorig:RightUpLeg"] {
            let leg_node = scene.graph.find_by_name(model, leg_name);
            layer_mask.merge(LayerMask::from_hierarchy(&scene.graph, leg_node));
        }
        root_layer.set_mask(layer_mask);

        let (
            walk_animation_resource,
            walk_pistol_animation_resource,
            idle_animation_resource,
            idle_pistol_animation_resource,
            jump_animation_resource,
            falling_animation_resource,
            landing_animation_resource,
            aim_rifle_animation_resource,
            aim_pistol_animation_resource,
            toss_grenade_animation_resource,
            put_back_animation_resource,
            grab_animation_resource,
            run_animation_resource,
            run_pistol_animation_resource,
            dying_animation_resource,
            hit_reaction_rifle_animation_resource,
            hit_reaction_pistol_animation_resource,
        ) = fyrox::core::futures::join!(
            resource_manager.request_model("data/animations/agent_walk_rifle.fbx"),
            resource_manager.request_model("data/animations/agent_idle_pistol.fbx"),
            resource_manager.request_model("data/animations/agent_idle.fbx"),
            resource_manager.request_model("data/animations/agent_idle_pistol.fbx"),
            resource_manager.request_model("data/animations/agent_jump.fbx"),
            resource_manager.request_model("data/animations/agent_falling.fbx"),
            resource_manager.request_model("data/animations/agent_landing.fbx"),
            resource_manager.request_model("data/animations/agent_aim_rifle.fbx"),
            resource_manager.request_model("data/animations/agent_aim_pistol.fbx"),
            resource_manager.request_model("data/animations/agent_toss_grenade.fbx"),
            resource_manager.request_model("data/animations/agent_put_back.fbx"),
            resource_manager.request_model("data/animations/agent_grab.fbx"),
            resource_manager.request_model("data/animations/agent_run_rifle.fbx"),
            resource_manager.request_model("data/animations/agent_run_pistol.fbx"),
            resource_manager.request_model("data/animations/agent_dying.fbx"),
            resource_manager.request_model("data/animations/agent_hit_reaction_rifle.fbx"),
            resource_manager.request_model("data/animations/agent_hit_reaction_pistol.fbx"),
        );

        let HitReactionStateDefinition {
            state: hit_reaction_state,
            hit_reaction_pistol_animation,
            hit_reaction_rifle_animation,
        } = make_hit_reaction_state(
            root_layer,
            scene,
            model,
            Self::HIT_REACTION_WEAPON_KIND.to_owned(),
            hit_reaction_rifle_animation_resource.unwrap(),
            hit_reaction_pistol_animation_resource.unwrap(),
            animation_player,
        );

        let aim_rifle_animation = *aim_rifle_animation_resource
            .unwrap()
            .retarget_animations(model, &mut scene.graph)
            .get(0)
            .unwrap();
        let aim_rifle_animation_node =
            root_layer.add_node(PoseNode::make_play_animation(aim_rifle_animation));

        let aim_pistol_animation = *aim_pistol_animation_resource
            .unwrap()
            .retarget_animations(model, &mut scene.graph)
            .get(0)
            .unwrap();
        let aim_pistol_animation_node =
            root_layer.add_node(PoseNode::make_play_animation(aim_pistol_animation));

        let aim_node = root_layer.add_node(PoseNode::make_blend_animations(vec![
            BlendPose::new(
                PoseWeight::Parameter(Self::RIFLE_AIM_FACTOR.to_owned()),
                aim_rifle_animation_node,
            ),
            BlendPose::new(
                PoseWeight::Parameter(Self::PISTOL_AIM_FACTOR.to_owned()),
                aim_pistol_animation_node,
            ),
        ]));
        let aim_state = root_layer.add_state(State::new("Aim", aim_node));

        let (toss_grenade_animation, toss_grenade_state) = create_play_animation_state(
            toss_grenade_animation_resource.unwrap(),
            "TossGrenade",
            root_layer,
            scene,
            model,
        );

        let IdleStateDefinition {
            state: idle_state, ..
        } = IdleStateDefinition::new(
            root_layer,
            scene,
            model,
            idle_animation_resource.unwrap(),
            idle_pistol_animation_resource.unwrap(),
            Self::IDLE_STATE_WEAPON_KIND.to_owned(),
            animation_player,
        );

        let (jump_animation, jump_state) = create_play_animation_state(
            jump_animation_resource.unwrap(),
            "Jump",
            root_layer,
            scene,
            model,
        );

        let (_, fall_state) = create_play_animation_state(
            falling_animation_resource.unwrap(),
            "Fall",
            root_layer,
            scene,
            model,
        );

        let (land_animation, land_state) = create_play_animation_state(
            landing_animation_resource.unwrap(),
            "Land",
            root_layer,
            scene,
            model,
        );

        let (put_back_animation, put_back_state) = create_play_animation_state(
            put_back_animation_resource.unwrap(),
            "PutBack",
            root_layer,
            scene,
            model,
        );

        let (grab_animation, grab_state) = create_play_animation_state(
            grab_animation_resource.unwrap(),
            "Grab",
            root_layer,
            scene,
            model,
        );

        let (dying_animation, dying_state) = create_play_animation_state(
            dying_animation_resource.unwrap(),
            "Dying",
            root_layer,
            scene,
            model,
        );

        let WalkStateDefinition {
            walk_animation,
            state: walk_state,
            run_animation,
            ..
        } = WalkStateDefinition::new(
            root_layer,
            scene,
            model,
            walk_animation_resource.unwrap(),
            walk_pistol_animation_resource.unwrap(),
            run_animation_resource.unwrap(),
            run_pistol_animation_resource.unwrap(),
            Self::WALK_STATE_WEAPON_KIND.to_owned(),
        );

        let animations_container =
            fetch_animation_container_mut(&mut scene.graph, animation_player);

        // Some animations must not be looped.
        animations_container
            .get_mut(jump_animation)
            .set_enabled(false)
            .set_loop(false);
        animations_container.get_mut(land_animation).set_loop(false);
        animations_container
            .get_mut(grab_animation)
            .set_loop(false)
            .set_speed(3.0)
            .set_enabled(false)
            .add_signal(AnimationSignal::new(
                Self::GRAB_WEAPON_SIGNAL,
                "GrabWeapon",
                0.3,
            ));
        let put_back_duration = animations_container.get(put_back_animation).length();
        animations_container
            .get_mut(put_back_animation)
            .set_speed(3.0)
            .add_signal(AnimationSignal::new(
                Self::PUT_BACK_WEAPON_END_SIGNAL,
                "PutBackWeapon",
                put_back_duration,
            ))
            .set_loop(false);
        animations_container
            .get_mut(toss_grenade_animation)
            .set_speed(1.5)
            .add_signal(AnimationSignal::new(
                Self::TOSS_GRENADE_SIGNAL,
                "TossGrenade",
                1.7,
            ))
            .set_enabled(false)
            .set_loop(false);

        animations_container
            .get_mut(dying_animation)
            .set_enabled(false)
            .set_loop(false);

        root_layer.add_transition(Transition::new(
            "Walk->Idle",
            walk_state,
            idle_state,
            0.30,
            Self::WALK_TO_IDLE,
        ));
        root_layer.add_transition(Transition::new(
            "Walk->Jump",
            walk_state,
            jump_state,
            0.20,
            Self::WALK_TO_JUMP,
        ));
        root_layer.add_transition(Transition::new(
            "Idle->Walk",
            idle_state,
            walk_state,
            0.40,
            Self::IDLE_TO_WALK,
        ));
        root_layer.add_transition(Transition::new(
            "Idle->Jump",
            idle_state,
            jump_state,
            0.25,
            Self::IDLE_TO_JUMP,
        ));
        root_layer.add_transition(Transition::new(
            "Falling->Landing",
            fall_state,
            land_state,
            0.20,
            Self::FALL_TO_LAND,
        ));
        root_layer.add_transition(Transition::new(
            "Landing->Idle",
            land_state,
            idle_state,
            0.20,
            Self::LAND_TO_IDLE,
        ));

        // Falling state can be entered from: Jump, Walk, Idle states.
        root_layer.add_transition(Transition::new(
            "Jump->Falling",
            jump_state,
            fall_state,
            0.30,
            Self::JUMP_TO_FALL,
        ));
        root_layer.add_transition(Transition::new(
            "Walk->Falling",
            walk_state,
            fall_state,
            0.30,
            Self::WALK_TO_FALL,
        ));
        root_layer.add_transition(Transition::new(
            "Idle->Falling",
            idle_state,
            fall_state,
            0.20,
            Self::IDLE_TO_FALL,
        ));
        root_layer.add_transition(Transition::new(
            "Idle->Aim",
            idle_state,
            aim_state,
            0.20,
            Self::IDLE_TO_AIM,
        ));
        root_layer.add_transition(Transition::new(
            "Walk->Aim",
            walk_state,
            aim_state,
            0.20,
            Self::WALK_TO_AIM,
        ));
        root_layer.add_transition(Transition::new(
            "Aim->Idle",
            aim_state,
            idle_state,
            0.20,
            Self::AIM_TO_IDLE,
        ));
        root_layer.add_transition(Transition::new(
            "Walk->Aim",
            aim_state,
            walk_state,
            0.20,
            Self::AIM_TO_WALK,
        ));
        root_layer.add_transition(Transition::new(
            "Aim->TossGrenade",
            aim_state,
            toss_grenade_state,
            0.20,
            Self::AIM_TO_TOSS_GRENADE,
        ));
        root_layer.add_transition(Transition::new(
            "TossGrenade->Aim",
            toss_grenade_state,
            aim_state,
            0.20,
            Self::TOSS_GRENADE_TO_AIM,
        ));

        root_layer.add_transition(Transition::new(
            "Aim->PutBack",
            aim_state,
            put_back_state,
            0.10,
            Self::AIM_TO_PUT_BACK,
        ));
        root_layer.add_transition(Transition::new(
            "Walk->PutBack",
            walk_state,
            put_back_state,
            0.10,
            Self::WALK_TO_PUT_BACK,
        ));
        root_layer.add_transition(Transition::new(
            "Idle->PutBack",
            idle_state,
            put_back_state,
            0.10,
            Self::IDLE_TO_PUT_BACK,
        ));

        root_layer.add_transition(Transition::new(
            "PutBack->Idle",
            put_back_state,
            idle_state,
            0.20,
            Self::PUT_BACK_TO_IDLE,
        ));
        root_layer.add_transition(Transition::new(
            "PutBack->Walk",
            put_back_state,
            walk_state,
            0.20,
            Self::PUT_BACK_TO_WALK,
        ));
        root_layer.add_transition(Transition::new(
            "PutBack->Grab",
            put_back_state,
            grab_state,
            0.10,
            Self::PUT_BACK_TO_GRAB,
        ));
        root_layer.add_transition(Transition::new(
            "Grab->Idle",
            grab_state,
            idle_state,
            0.20,
            Self::GRAB_TO_IDLE,
        ));
        root_layer.add_transition(Transition::new(
            "Grab->Walk",
            grab_state,
            walk_state,
            0.20,
            Self::GRAB_TO_WALK,
        ));
        root_layer.add_transition(Transition::new(
            "Grab->Aim",
            grab_state,
            aim_state,
            0.20,
            Self::GRAB_TO_AIM,
        ));

        // Dying transitions.
        root_layer.add_transition(Transition::new(
            "Land->Dying",
            land_state,
            dying_state,
            0.20,
            Self::LAND_TO_DYING,
        ));
        root_layer.add_transition(Transition::new(
            "Fall->Dying",
            fall_state,
            dying_state,
            0.20,
            Self::FALL_TO_DYING,
        ));
        root_layer.add_transition(Transition::new(
            "Idle->Dying",
            idle_state,
            dying_state,
            0.20,
            Self::IDLE_TO_DYING,
        ));
        root_layer.add_transition(Transition::new(
            "Walk->Dying",
            walk_state,
            dying_state,
            0.20,
            Self::WALK_TO_DYING,
        ));
        root_layer.add_transition(Transition::new(
            "Jump->Dying",
            jump_state,
            dying_state,
            0.20,
            Self::JUMP_TO_DYING,
        ));
        root_layer.add_transition(Transition::new(
            "Aim->Dying",
            aim_state,
            dying_state,
            0.20,
            Self::AIM_TO_DYING,
        ));
        root_layer.add_transition(Transition::new(
            "TossGrenade->Dying",
            toss_grenade_state,
            dying_state,
            0.20,
            Self::TOSS_GRENADE_TO_DYING,
        ));
        root_layer.add_transition(Transition::new(
            "Grab->Dying",
            grab_state,
            dying_state,
            0.20,
            Self::GRAB_TO_DYING,
        ));
        root_layer.add_transition(Transition::new(
            "PutBack->Dying",
            put_back_state,
            dying_state,
            0.20,
            Self::PUT_BACK_TO_DYING,
        ));

        root_layer.add_transition(Transition::new(
            "Idle->HitReaction",
            idle_state,
            hit_reaction_state,
            0.20,
            Self::IDLE_TO_HIT_REACTION,
        ));
        root_layer.add_transition(Transition::new(
            "Walk->HitReaction",
            walk_state,
            hit_reaction_state,
            0.20,
            Self::WALK_TO_HIT_REACTION,
        ));
        root_layer.add_transition(Transition::new(
            "HitReaction->Idle",
            hit_reaction_state,
            idle_state,
            0.20,
            Self::HIT_REACTION_TO_IDLE,
        ));
        root_layer.add_transition(Transition::new(
            "HitReaction->Walk",
            hit_reaction_state,
            walk_state,
            0.20,
            Self::HIT_REACTION_TO_WALK,
        ));
        root_layer.add_transition(Transition::new(
            "HitReaction->Dying",
            hit_reaction_state,
            dying_state,
            0.20,
            Self::HIT_REACTION_TO_DYING,
        ));

        root_layer.add_transition(Transition::new(
            "Aim->HitReaction",
            aim_state,
            hit_reaction_state,
            0.20,
            Self::AIM_TO_HIT_REACTION,
        ));
        root_layer.add_transition(Transition::new(
            "HitReaction->Aim",
            hit_reaction_state,
            aim_state,
            0.20,
            Self::HIT_REACTION_TO_AIM,
        ));

        root_layer.set_entry_state(idle_state);

        Self {
            machine,
            aim_state,
            toss_grenade_state,
            put_back_state,
            jump_animation,
            walk_animation,
            run_animation,
            land_animation,
            toss_grenade_animation,
            put_back_animation,
            grab_animation,
            dying_animation,
            hit_reaction_pistol_animation,
            hit_reaction_rifle_animation,
        }
    }

    pub fn apply(
        &mut self,
        scene: &mut Scene,
        dt: f32,
        hips_handle: Handle<Node>,
        input: UpperBodyMachineInput,
        animation_player: Handle<Node>,
    ) {
        let animations_container = fetch_animation_container_ref(&scene.graph, animation_player);

        let (current_hit_reaction_animation, index) = match input.weapon_kind {
            CombatWeaponKind::Rifle => (self.hit_reaction_rifle_animation, 0),
            CombatWeaponKind::Pistol => (self.hit_reaction_pistol_animation, 1),
        };
        let recovered = !input.should_be_stunned
            && animations_container[current_hit_reaction_animation].has_ended();

        self.machine
            // Update parameters which will be used by transitions.
            .set_parameter(Self::IDLE_TO_WALK, Parameter::Rule(input.is_walking))
            .set_parameter(Self::WALK_TO_IDLE, Parameter::Rule(!input.is_walking))
            .set_parameter(Self::WALK_TO_JUMP, Parameter::Rule(input.is_jumping))
            .set_parameter(Self::IDLE_TO_JUMP, Parameter::Rule(input.is_jumping))
            .set_parameter(
                Self::JUMP_TO_FALL,
                Parameter::Rule(animations_container.get(self.jump_animation).has_ended()),
            )
            .set_parameter(
                Self::WALK_TO_FALL,
                Parameter::Rule(!input.has_ground_contact),
            )
            .set_parameter(
                Self::IDLE_TO_FALL,
                Parameter::Rule(!input.has_ground_contact),
            )
            .set_parameter(
                Self::FALL_TO_LAND,
                Parameter::Rule(input.has_ground_contact),
            )
            .set_parameter(
                Self::LAND_TO_IDLE,
                Parameter::Rule(animations_container.get(self.land_animation).has_ended()),
            )
            .set_parameter(
                Self::IDLE_TO_AIM,
                Parameter::Rule(input.is_aiming && input.has_ground_contact),
            )
            .set_parameter(
                Self::WALK_TO_AIM,
                Parameter::Rule(input.is_aiming && input.has_ground_contact),
            )
            .set_parameter(
                Self::AIM_TO_IDLE,
                Parameter::Rule(!input.is_aiming || !input.has_ground_contact),
            )
            .set_parameter(
                Self::AIM_TO_WALK,
                Parameter::Rule(!input.is_aiming || !input.has_ground_contact),
            )
            .set_parameter(
                Self::AIM_TO_PUT_BACK,
                Parameter::Rule(input.is_aiming && input.change_weapon),
            )
            .set_parameter(Self::WALK_TO_PUT_BACK, Parameter::Rule(input.change_weapon))
            .set_parameter(Self::IDLE_TO_PUT_BACK, Parameter::Rule(input.change_weapon))
            .set_parameter(
                Self::PUT_BACK_TO_IDLE,
                Parameter::Rule(
                    !input.change_weapon
                        && animations_container
                            .get(self.put_back_animation)
                            .has_ended(),
                ),
            )
            .set_parameter(
                Self::PUT_BACK_TO_WALK,
                Parameter::Rule(
                    !input.change_weapon
                        && input.is_walking
                        && animations_container
                            .get(self.put_back_animation)
                            .has_ended(),
                ),
            )
            .set_parameter(
                Self::PUT_BACK_TO_GRAB,
                Parameter::Rule(
                    input.change_weapon
                        && animations_container
                            .get(self.put_back_animation)
                            .has_ended(),
                ),
            )
            .set_parameter(
                Self::GRAB_TO_IDLE,
                Parameter::Rule(
                    !input.change_weapon
                        && !input.is_aiming
                        && animations_container.get(self.grab_animation).has_ended(),
                ),
            )
            .set_parameter(
                Self::GRAB_TO_WALK,
                Parameter::Rule(
                    !input.change_weapon
                        && input.is_walking
                        && !input.is_aiming
                        && animations_container.get(self.grab_animation).has_ended(),
                ),
            )
            .set_parameter(
                Self::GRAB_TO_AIM,
                Parameter::Rule(
                    input.is_aiming && animations_container.get(self.grab_animation).has_ended(),
                ),
            )
            .set_parameter(
                Self::PISTOL_AIM_FACTOR,
                Parameter::Weight(if input.weapon_kind == CombatWeaponKind::Pistol {
                    1.0
                } else {
                    0.0
                }),
            )
            .set_parameter(
                Self::RIFLE_AIM_FACTOR,
                Parameter::Weight(if input.weapon_kind == CombatWeaponKind::Rifle {
                    1.0
                } else {
                    0.0
                }),
            )
            .set_parameter(Self::HIT_REACTION_WEAPON_KIND, Parameter::Index(index))
            .set_parameter(
                Self::IDLE_TO_HIT_REACTION,
                Parameter::Rule(input.should_be_stunned),
            )
            .set_parameter(
                Self::WALK_TO_HIT_REACTION,
                Parameter::Rule(input.should_be_stunned),
            )
            .set_parameter(
                Self::AIM_TO_HIT_REACTION,
                Parameter::Rule(input.should_be_stunned),
            )
            .set_parameter(Self::HIT_REACTION_TO_IDLE, Parameter::Rule(recovered))
            .set_parameter(Self::HIT_REACTION_TO_WALK, Parameter::Rule(recovered))
            .set_parameter(Self::HIT_REACTION_TO_DYING, Parameter::Rule(input.is_dead))
            .set_parameter(
                Self::HIT_REACTION_TO_AIM,
                Parameter::Rule(recovered && input.is_aiming),
            )
            .set_parameter(Self::LAND_TO_DYING, Parameter::Rule(input.is_dead))
            .set_parameter(Self::IDLE_TO_DYING, Parameter::Rule(input.is_dead))
            .set_parameter(Self::FALL_TO_DYING, Parameter::Rule(input.is_dead))
            .set_parameter(Self::WALK_TO_DYING, Parameter::Rule(input.is_dead))
            .set_parameter(Self::JUMP_TO_DYING, Parameter::Rule(input.is_dead))
            .set_parameter(Self::AIM_TO_DYING, Parameter::Rule(input.is_dead))
            .set_parameter(Self::TOSS_GRENADE_TO_DYING, Parameter::Rule(input.is_dead))
            .set_parameter(Self::GRAB_TO_DYING, Parameter::Rule(input.is_dead))
            .set_parameter(Self::PUT_BACK_TO_DYING, Parameter::Rule(input.is_dead))
            .set_parameter(
                Self::WALK_STATE_WEAPON_KIND,
                Parameter::Index(index + if input.run_factor > 0.1 { 2 } else { 0 }),
            )
            .set_parameter(
                Self::TOSS_GRENADE_TO_AIM,
                Parameter::Rule(
                    !input.toss_grenade
                        && animations_container
                            .get(self.toss_grenade_animation)
                            .has_ended(),
                ),
            )
            .set_parameter(
                Self::AIM_TO_TOSS_GRENADE,
                Parameter::Rule(input.toss_grenade && input.is_aiming),
            )
            .set_parameter(Self::IDLE_STATE_WEAPON_KIND, Parameter::Index(index))
            .evaluate_pose(animations_container, dt)
            .apply_with(&mut scene.graph, |node, handle, pose| {
                if handle == hips_handle {
                    // Ignore position and rotation for hips. Some animations has unwanted shifts
                    // and we want to ignore them.
                    pose.values()
                        .values
                        .iter()
                        .filter_map(|v| {
                            if v.binding == ValueBinding::Scale {
                                if let TrackValue::Vector3(vec3) = v.value {
                                    Some(vec3)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .for_each(|v| {
                            node.local_transform_mut().set_scale(v);
                        })
                } else {
                    pose.values().apply(node);
                }
            });
    }

    pub fn hit_reaction_animations(&self) -> [Handle<Animation>; 2] {
        [
            self.hit_reaction_rifle_animation,
            self.hit_reaction_pistol_animation,
        ]
    }
}
