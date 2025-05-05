// src/app/combat_engine.rs

use std::collections::HashMap;
use crate::app::card_attribute::KeywordAbility;
use crate::app::card_library::Creature;
use crate::app::card_attribute::PlayerSelector;

/// Internal representation of a creature in combat
#[derive(Debug)]
struct CombatCreature {
    pub power: i32,
    pub toughness: i32,
    pub damage_taken: i32,
    pub abilities: Vec<KeywordAbility>,
    pub controller: PlayerSelector,
}

/// A combat pairing: an attacker index and its blockers
#[derive(Debug)]
struct CombatGroup {
    attacker: usize,
    blockers: Vec<usize>,
}

/// Central engine for combat resolution
pub struct CombatEngine;

impl CombatEngine {
    /// Resolves combat given attackers, attacker creatures, blockers, and prevents lifegain flag.
    /// Returns (surviving_attackers, surviving_blockers, unblocked_damage, life_gain)
    pub fn resolve_combat(
        attackers: &[usize],
        attack_side: &[Creature],
        block_side: &[Creature],
        blocks: &HashMap<usize, Vec<usize>>,
        prevent_lifegain: &mut bool,
    ) -> (Vec<bool>, Vec<bool>, i32, i32) {
        // 1) Build internal creature list
        let mut creatures = Vec::new();
        for cr in attack_side.iter() {
            creatures.push(CombatCreature {
                power: cr.power,
                toughness: cr.toughness,
                damage_taken: 0,
                abilities: cr.abilities.clone(),
                controller: PlayerSelector::Controller,
            });
        }
        let offset = creatures.len();
        for cr in block_side.iter() {
            creatures.push(CombatCreature {
                power: cr.power,
                toughness: cr.toughness,
                damage_taken: 0,
                abilities: cr.abilities.clone(),
                controller: PlayerSelector::Opponent,
            });
        }

        // 2) Build groups of attacker + its blockers
        let mut groups = Vec::new();
        for &atk in attackers {
            let blk_idxs = blocks
                .get(&atk)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|b| offset + b)
                .collect();
            groups.push(CombatGroup { attacker: atk, blockers: blk_idxs });
        }

        // 3) First Strike / Double Strike first hits
        let mut unblocked_dmg = 0;
        for grp in &groups {
            // attacker first-strike
            if grp.blockers.is_empty() {
                // direct to opponent if has first-strike or double-strike
                if creatures[grp.attacker]
                    .abilities
                    .contains(&KeywordAbility::FirstStrike)
                    || creatures[grp.attacker]
                    .abilities
                    .contains(&KeywordAbility::DoubleStrike)
                {
                    unblocked_dmg += creatures[grp.attacker].power;
                }
            } else if creatures[grp.attacker]
                .abilities
                .contains(&KeywordAbility::FirstStrike)
                || creatures[grp.attacker]
                .abilities
                .contains(&KeywordAbility::DoubleStrike)
            {
                // deal damage to blockers in order
                let mut rem = creatures[grp.attacker].power;
                for &blk in &grp.blockers {
                    if rem <= 0 { break; }
                    let needed = if creatures[grp.attacker].abilities.contains(&KeywordAbility::Deathtouch) {
                        1
                    } else {
                        creatures[blk].toughness - creatures[blk].damage_taken
                    };
                    let dmg = rem.min(needed);
                    creatures[blk].damage_taken += dmg;
                    rem -= dmg;
                }
            }
            // blockers first-strike
            for &blk in &grp.blockers {
                if creatures[blk]
                    .abilities
                    .contains(&KeywordAbility::FirstStrike)
                    || creatures[blk]
                    .abilities
                    .contains(&KeywordAbility::DoubleStrike)
                {
                    creatures[grp.attacker].damage_taken += creatures[blk].power;
                }
            }
        }

        // 4) Normal / second strike hits
        for grp in &groups {
            let fs_only = creatures[grp.attacker]
                .abilities
                .contains(&KeywordAbility::FirstStrike)
                && !creatures[grp.attacker]
                .abilities
                .contains(&KeywordAbility::DoubleStrike);
            if !fs_only {
                if grp.blockers.is_empty() {
                    unblocked_dmg += creatures[grp.attacker].power;
                } else {
                    let mut rem = creatures[grp.attacker].power;
                    for &blk in &grp.blockers {
                        if rem <= 0 { break; }
                        let needed = if creatures[grp.attacker].abilities.contains(&KeywordAbility::Deathtouch) {
                            1
                        } else {
                            creatures[blk].toughness - creatures[blk].damage_taken
                        };
                        let dmg = rem.min(needed);
                        creatures[blk].damage_taken += dmg;
                        rem -= dmg;
                    }
                }
            }
            // blockers normal strike
            for &blk in &grp.blockers {
                let blk_fs_only = creatures[blk]
                    .abilities
                    .contains(&KeywordAbility::FirstStrike)
                    && !creatures[blk]
                    .abilities
                    .contains(&KeywordAbility::DoubleStrike);
                if !blk_fs_only {
                    creatures[grp.attacker].damage_taken += creatures[blk].power;
                }
            }
        }

        // 5) Determine survivors and total lifegain
        let mut survives_att = Vec::new();
        let mut survives_blk = Vec::new();
        let mut life_gain = 0;
        for (i, c) in creatures.iter().enumerate() {
            let alive = c.damage_taken < c.toughness;
            if i < offset {
                survives_att.push(alive);
            } else {
                survives_blk.push(alive);
            }
            // lifelink: gain life equal to damage_dealt
            if c.abilities.contains(&KeywordAbility::Lifelink) {
                life_gain += c.damage_taken;
            }
        }

        // 6) Apply prevent lifegain
        let actual_gain = if *prevent_lifegain {
            *prevent_lifegain = false;
            0
        } else {
            life_gain
        };

        (survives_att, survives_blk, unblocked_dmg, actual_gain)
    }
}
