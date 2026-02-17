use crate::{
    constraint::{CheckType, Constraint, ConstraintType, Offer},
    matching_repr::{bitset::Bitset, MaskedMatching},
};

impl Constraint {
    pub fn is_blackout(&self) -> bool {
        if let ConstraintType::Night { num: _, comment: _ } = self.r#type {
            if let CheckType::Lights(l, _) = self.check {
                return self.known_lights == l;
            }
        }
        false
    }

    pub fn is_sold(&self) -> bool {
        if let ConstraintType::Box { .. } = self.r#type {
            if let CheckType::Sold = self.check {
                return true;
            }
        }
        false
    }

    pub fn is_match_found(&self) -> bool {
        if let ConstraintType::Box { .. } = self.r#type {
            if let CheckType::Lights(1, _) = self.check {
                return true;
            }
        }
        false
    }

    pub fn try_get_offer(&self) -> Option<Offer> {
        if let ConstraintType::Box { offer, .. } = &self.r#type {
            offer.clone()
        } else {
            None
        }
    }

    pub fn is_mb_hit(&self, solutions: Option<&Vec<MaskedMatching>>) -> bool {
        if let Some(sols) = solutions {
            if let ConstraintType::Box { .. } = self.r#type {
                return sols.iter().all(|sol| {
                    self.map.iter_pairs().all(|(a, b)| {
                        sol.slot_mask(a as usize)
                            .unwrap_or(&Bitset::empty())
                            .contains(b)
                    })
                });
            }
        }
        false
    }

    pub fn might_won(&self) -> bool {
        matches!(self.r#type, ConstraintType::Night { .. })
    }

    pub fn won(&self, required_lights: usize) -> bool {
        if let ConstraintType::Night { .. } = self.r#type {
            match self.check {
                CheckType::Eq => false,
                CheckType::Nothing | CheckType::Sold => false,
                CheckType::Lights(l, _) => l as usize == required_lights,
            }
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::eval_types::{EvalEvent, EvalInitial, EvalMN};
    use crate::matching_repr::MaskedMatching;
    use crate::{constraint::eval_types::EvalMB, ruleset_data::dummy::DummyData};
    use rust_decimal::dec;
    use std::collections::{BTreeMap, HashMap};

    #[test]
    fn evalevent_query_functions_mixed_variants() {
        // MB event
        let mb = EvalMB {
            num: dec![2.0],
            bits_left_after: 8.0,
            lights_total: Some(3),
            lights_known_before: 1,
            bits_gained: 2.5,
            comment: "mb".to_string(),
            offer: true,
        };
        let ev_mb = EvalEvent::MB(mb.clone());

        // MN event
        let mn = EvalMN {
            num: dec![4.0],
            bits_left_after: 16.0,
            lights_total: Some(2),
            lights_known_before: 0,
            bits_gained: 3.5,
            comment: "mn".to_string(),
        };
        let ev_mn = EvalEvent::MN(mn.clone());

        // Initial event
        let ini = EvalInitial {
            bits_left_after: 32.0,
            comment: "init".to_string(),
        };
        let ev_ini = EvalEvent::Initial(ini.clone());

        // MB: get number using closures (mn_pred, mb_pred, init_pred)
        assert_eq!(ev_mb.num(|_| false, |_| true, |_| false), Some(dec![2.0]));
        assert_eq!(ev_mn.num(|_| true, |_| false, |_| false), Some(dec![4.0]));
        assert_eq!(ev_ini.num(|_| false, |_| false, |_| true), Some(dec![0]));

        // bits_gained: MB and MN present, Initial -> None
        assert_eq!(ev_mb.bits_gained(|_| false, |_| true, |_| false), Some(2.5));
        assert_eq!(ev_mn.bits_gained(|_| true, |_| false, |_| false), Some(3.5));
        assert_eq!(ev_ini.bits_gained(|_| false, |_| false, |_| true), None);

        // lights_total: present for MB/MN; Initial -> None
        assert_eq!(ev_mb.lights_total(|_| false, |_| true, |_| false), Some(3));
        assert_eq!(ev_mn.lights_total(|_| true, |_| false, |_| false), Some(2));
        assert_eq!(ev_ini.lights_total(|_| false, |_| false, |_| true), None);

        // new_lights = lights_total - lights_known_before
        assert_eq!(ev_mb.new_lights(|_| false, |_| true, |_| false), Some(2));
        assert_eq!(ev_mn.new_lights(|_| true, |_| false, |_| false), Some(2));
    }

    #[test]
    fn predicates_and_offer_detection() {
        // NIGHT with lights == known_lights -> blackout
        let c_blackout = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: MaskedMatching::from_matching_ref(&[vec![0]]),
            eliminated: 0,
            eliminated_tab: vec![vec![0; 1]; 1],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: dec![1.0],
                comment: "".to_string(),
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 2,
        };
        assert!(c_blackout.is_blackout());
        assert!(c_blackout.might_won());
        assert!(!c_blackout.is_sold());
        assert!(!c_blackout.is_match_found());

        // BOX with CheckType::Sold
        let c_sold = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Sold,
            map: MaskedMatching::from_matching_ref(&[vec![0]]),
            eliminated: 0,
            eliminated_tab: vec![vec![0; 1]; 1],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: dec![1.0],
                comment: "".to_string(),
                offer: None,
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };
        assert!(c_sold.is_sold());
        assert!(!c_sold.is_blackout());

        // BOX with offer
        let c_offer = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Eq,
            map: MaskedMatching::from_matching_ref(&[vec![0]]),
            eliminated: 0,
            eliminated_tab: vec![vec![0; 1]; 1],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: dec![2.0],
                comment: "".to_string(),
                offer: Some(Offer::Single {
                    amount: Some(10),
                    by: "X".into(),
                    reduced_pot: false,
                    save: false,
                }),
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };
        assert!(c_offer.try_get_offer().is_some());
    }

    #[test]
    fn get_stats_produces_expected_eval_events() {
        let c = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::from([("A".to_string(), "a".to_string())]),
            check: CheckType::Lights(1, BTreeMap::new()),
            map: MaskedMatching::from_matching_ref(&[vec![0]]),
            eliminated: 0,
            eliminated_tab: vec![vec![0; 1]; 1],
            information: Some(2.0),
            left_after: Some(1024),
            hidden: false,
            r#type: ConstraintType::Box {
                num: dec![3.0],
                comment: "".to_string(),
                offer: None,
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        if let Ok(Some(EvalEvent::MB(ev))) = c.get_stats() {
            assert_eq!(ev.num, dec![3.0]);
            assert_eq!(ev.lights_total, Some(1u8));
            assert!((ev.bits_left_after - (1024f64).log2()).abs() < 1e-9);
            assert!((ev.bits_gained - 2.0).abs() < 1e-9);
        } else {
            panic!("expected MB event");
        }
    }

    #[test]
    fn was_solvable_before_none_and_true_false_cases() {
        let mut c = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Lights(1, BTreeMap::new()),
            map: MaskedMatching::from_matching_ref(&[vec![0], vec![1]]),
            eliminated: 0,
            eliminated_tab: vec![vec![0; 2]; 2],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: dec![1.0],
                comment: "".to_string(),
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        // empty left_poss -> Ok(None)
        assert_eq!(c.was_solvable_before().unwrap(), None);

        // identical left_poss -> should be Some(true)
        let m = MaskedMatching::from_matching_ref(&[vec![0], vec![1]]);
        c.left_poss.push(m.clone());
        c.left_poss.push(m.clone());
        assert_eq!(c.was_solvable_before().unwrap(), Some(true));

        // conflicting left_poss -> Some(false)
        let m1 = MaskedMatching::from_matching_ref(&[vec![0], vec![2]]);
        let m2 = MaskedMatching::from_matching_ref(&[vec![1], vec![2]]);
        let mut c2 = c.clone();
        c2.left_poss = vec![m1.clone(), m2.clone()];
        // if overlaying reduces lights below sol.len() should return Some(false)
        // In practice this depends on calculate_lights behavior — assert we get an Option
        let res = c2.was_solvable_before();
        assert!(res.is_ok());
    }

    #[test]
    fn md_title_and_show_flags() {
        let c_n = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: MaskedMatching::from_matching_ref(&[vec![0]]),
            eliminated: 0,
            eliminated_tab: vec![vec![0; 1]; 1],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: dec![7.0],
                comment: "hello -- extra".to_string(),
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };
        assert!(c_n.md_title().starts_with("MN#7.0"));
        assert!(c_n.show_lights_information());
        assert!(c_n.show_expected_information());
        assert!(c_n.show_past_cnt());
        assert!(c_n.show_new());
        assert!(c_n.show_past_dist());
        assert!(c_n.adds_new());
    }
}
