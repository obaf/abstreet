use ezgui::input::UserInput;
use map_model::{EditReason, Edits, LaneID, LaneType, Map};
use piston::input::Key;
use plugins::selection::SelectionState;
use render::DrawMap;
use sim::Sim;

pub enum RoadEditor {
    Inactive(Edits),
    Active(Edits),
}

impl RoadEditor {
    pub fn new(edits: Edits) -> RoadEditor {
        RoadEditor::Inactive(edits)
    }

    pub fn event(
        &mut self,
        input: &mut UserInput,
        current_selection: &SelectionState,
        map: &mut Map,
        draw_map: &mut DrawMap,
        sim: &mut Sim,
    ) -> bool {
        let mut new_state: Option<RoadEditor> = None;
        // TODO a bit awkward that we can't pull this info from Edits easily
        let mut changed: Option<(LaneID, LaneType)> = None;

        let active = match self {
            RoadEditor::Inactive(edits) => match current_selection {
                SelectionState::Empty => {
                    if input.unimportant_key_pressed(Key::E, "Start editing roads") {
                        // TODO cloning edits sucks! want to consume self
                        new_state = Some(RoadEditor::Active(edits.clone()));
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            RoadEditor::Active(edits) => {
                if input.key_pressed(Key::Return, "Press enter to stop editing roads") {
                    new_state = Some(RoadEditor::Inactive(edits.clone()));
                } else if let SelectionState::SelectedLane(id, _) = *current_selection {
                    let lane = map.get_l(id);
                    let road = map.get_r(lane.parent);
                    let reason = EditReason::BasemapWrong; // TODO be able to choose

                    if lane.lane_type != LaneType::Driving
                        && input.key_pressed(Key::D, "Press D to make this a driving lane")
                    {
                        if edits.change_lane_type(reason, road, lane, LaneType::Driving) {
                            changed = Some((lane.id, LaneType::Driving));
                        }
                    }
                    if lane.lane_type != LaneType::Parking
                        && input.key_pressed(Key::P, "Press p to make this a parking lane")
                    {
                        if edits.change_lane_type(reason, road, lane, LaneType::Parking) {
                            changed = Some((lane.id, LaneType::Parking));
                        }
                    }
                    if lane.lane_type != LaneType::Biking
                        && input.key_pressed(Key::B, "Press b to make this a bike lane")
                    {
                        if edits.change_lane_type(reason, road, lane, LaneType::Biking) {
                            changed = Some((lane.id, LaneType::Biking));
                        }
                    }
                    if input.key_pressed(Key::Backspace, "Press backspace to delete this lane") {
                        if edits.delete_lane(road, lane) {
                            println!("Have to reload the map from scratch to pick up this change!");
                        }
                    }
                }

                true
            }
        };
        if let Some(s) = new_state {
            *self = s;
        }
        if let Some((id, new_type)) = changed {
            let intersections = map.get_l(id).intersections();

            // TODO generally tense about having two methods to carry out this change. weird
            // intermediate states are scary. maybe pass old and new struct for intersection (aka
            // list of turns)?

            // Remove turns
            for i in &intersections {
                for t in &map.get_i(*i).turns {
                    draw_map.edit_remove_turn(*t);
                    sim.edit_remove_turn(map.get_t(*t));
                }
            }

            let old_type = map.get_l(id).lane_type;
            map.edit_lane_type(id, new_type);
            draw_map.edit_lane_type(id, map);
            sim.edit_lane_type(id, old_type, map);

            // Add turns back
            for i in &intersections {
                for t in &map.get_i(*i).turns {
                    draw_map.edit_add_turn(*t, map);
                    sim.edit_add_turn(map.get_t(*t), map);
                }
            }
        }
        active
    }

    pub fn get_edits(&self) -> &Edits {
        match self {
            RoadEditor::Inactive(edits) => edits,
            RoadEditor::Active(edits) => edits,
        }
    }
}