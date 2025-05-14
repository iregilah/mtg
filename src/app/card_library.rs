use std::collections::HashMap;
use crate::app::card_attribute::*;
use crate::app::card_attribute::CardAttribute;
use crate::app::gre::gre_structs::ActivatedAbility;
use crate::app::game_state::GamePhase;
use std::hash::{Hash, Hasher};
use bitflags::bitflags;
use tracing::debug;
use crate::app::card_attribute::CreatureType::Detective;

const CACOPHONY_SCAMP: &str = "Cacophony Scamp";
const MONASTERY_SWIFTSPEAR: &str = "Monastery Swiftspear";
const ELECTROSTATIC_INFANTRY: &str = "Electrostatic Infantry";
const HEARTFIRE_HERO: &str = "Heartfire Hero";
const FELONIOUS_RAGE: &str = "Felonious Rage";
const MONSTROUS_RAGE: &str = "Monstrous Rage";
const MONSTER_ROLE: &str = "Monster Role";
const BLAZING_CRESCENDO: &str = "Blazing Crescendo";
const DEMONIC_RUCKUS: &str = "Demonic Ruckus";
const BURST_LIGHTNING: &str = "Burst Lightning";
const LIGHTNING_STRIKE: &str = "Lightning Strike";
const MOUNTAIN: &str = "Mountain";
const ROCKFACE_VILLAGE: &str = "Rockface Village";


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Creature {
    pub power: i32,
    pub toughness: i32,
    pub summoning_sickness: bool,
    pub abilities: Vec<KeywordAbility>,
    pub types: Vec<CreatureType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CardType {
    Creature(Creature),
    Instant,
    Land,
    Enchantment,
    Token,
}

/// ManaCost
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ManaCost {
    pub colorless: u32,
    pub red: u32,
    pub green: u32,
    pub blue: u32,
    pub white: u32,
    pub black: u32,
}

impl ManaCost {
    pub fn new(colorless: u32, red: u32, green: u32, blue: u32, white: u32, black: u32) -> Self {
        ManaCost { colorless, red, green, blue, white, black }
    }
    pub fn free() -> Self {
        ManaCost::new(0, 0, 0, 0, 0, 0)
    }
    pub fn total(&self) -> u32 {
        self.colorless + self.red + self.green + self.blue + self.white + self.black
    }
    pub fn colored(&self) -> u32 {
        self.red + self.green + self.blue + self.white + self.black
    }
}
bitflags! {
    #[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
    pub struct CardTypeFlags: u32 {
        const NONE         = 0;
        const LAND         = 1 << 0;
        const CREATURE     = 1 << 1;
        const INSTANT      = 1 << 2;
        const SORCERY      = 1 << 3;
        const ENCHANTMENT  = 1 << 4;
        const ARTIFACT     = 1 << 5;
        const PLANESWALKER = 1 << 6;
        const BATTLE       = 1 << 7;
        const TOKEN        = 1 << 8;
    }
}
/// A kártya fő struktúrája.
/// + `type_flags` mező is, bitflags-alapú
#[derive(Debug, Clone)]
pub struct Card {
    pub card_id: u64,
    pub name: String,
    pub card_type: CardType,
    pub type_flags: CardTypeFlags,
    pub mana_cost: ManaCost,
    pub attributes: Vec<Box<dyn CardAttribute>>,
    pub triggers: Vec<Trigger>,
    pub activated_abilities: Vec<ActivatedAbility>,
    pub attached_to: Option<u64>,
}
impl Hash for Card {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // például:
        self.card_id.hash(state);
        self.name.hash(state);

        // card_type -> Hash
        //   Ehhez lentebb "CardType" is implementálja a Hash-t
        self.card_type.hash(state);

        // type_flags bitjei
        self.type_flags.bits().hash(state);

        // mana_cost -> #derive(Hash)
        self.mana_cost.hash(state);

        // DÖNTÉS: triggers & attributes is hashelődjön?
        // Ehhez a Trigger, Condition, SpellFilter, PlayerSelector, stb. mind kell #derive(Hash).
        // Viszont a `Box<dyn CardAttribute>`
        //  nem derívelhető automatikusan.
        //    -> vagy kihagyod, vagy custom logikát írsz.
        // self.triggers.hash(state);   // ha Trigger is #derive(Hash)
        // self.activated_abilities.hash(state); // ha ActivatedAbility is #derive(Hash)

        // attributes : Vec<Box<dyn CardAttribute>>
        //   Erre tipikusan NINCS egyszerű derive-hash.
        //   Vagy teljesen kihagyod, vagy mindegyik CardAttribute típushoz
        //   egyedi Hash-implementációt írsz, + dyn-diszpatch.

        // attached_to
        self.attached_to.hash(state);
    }
}

impl PartialEq for Card {
    fn eq(&self, other: &Self) -> bool {
        // Elég, ha ezek megegyeznek (attributes/activated_abilities-t NEM hasonlítjuk)
        self.name == other.name
            && self.card_type == other.card_type
            && self.type_flags == other.type_flags
            && self.mana_cost == other.mana_cost
            && self.triggers == other.triggers
        // attributes és activated_abilities kimarad
    }
}

impl Eq for Card {}

impl Card {
    /// Létrehoz egy új kártyát, a `card_type` alapján
    /// automatikusan beállítva a `type_flags` mezőt is.
    pub fn new(name: &str, card_type: CardType, mana_cost: ManaCost) -> Self {
        let mut flags = CardTypeFlags::NONE;
        match &card_type {
            CardType::Creature(_) => { flags |= CardTypeFlags::CREATURE; }
            CardType::Instant => { flags |= CardTypeFlags::INSTANT; }
            CardType::Enchantment => { flags |= CardTypeFlags::ENCHANTMENT; }
            CardType::Land => { flags |= CardTypeFlags::LAND; }
            CardType::Token => { flags |= CardTypeFlags::TOKEN; }
        }
        Card {
            card_id: 0, // GRE osztja ki
            name: name.into(),
            card_type,
            type_flags: flags,
            mana_cost,
            attributes: Vec::new(),
            triggers: Vec::new(),
            activated_abilities: Vec::new(),
            attached_to: None,
        }
    }
    /// Egy triggert és attribútumot ad a kártyához.
    pub fn with(mut self, trigger: Trigger, attr: impl CardAttribute + 'static) -> Self {
        self.triggers.push(trigger);
        self.attributes.push(Box::new(attr));
        self
    }
    /// Egy activated ability-t is hozzáadhatunk
    pub fn with_activated(mut self, ability: ActivatedAbility) -> Self {
        self.activated_abilities.push(ability);
        self
    }

    /// A token bitflag vagy bármely más bitflag hozzáadása
    pub fn with_added_type(mut self, flag: CardTypeFlags) -> Self {
        self.type_flags |= flag;
        self
    }
    /// A creature powerjét átírja (ha creature)
    pub fn with_power(mut self, new_power: i32) -> Self {
        if let CardType::Creature(ref mut c) = self.card_type {
            c.power = new_power;
        }
        self
    }
    /// A creature toughness-ét átírja (ha creature)
    pub fn with_toughness(mut self, new_toughness: i32) -> Self {
        if let CardType::Creature(ref mut c) = self.card_type {
            c.toughness = new_toughness;
        }
        self
    }

    /// A kártyán lévő attribute-öket/trigger-öket futtatjuk le,
    /// megnézve, illik-e a paraméter `trigger`-re.
    pub fn trigger_by(&mut self, trigger: &Trigger) -> Vec<Effect> {
        self.triggers
            .iter()
            .zip(self.attributes.iter_mut())
            .filter_map(|(t, attr)| {
                if t == trigger {
                    attr.on_trigger(trigger)
                } else {
                    None
                }
            })
            .collect()
    }
    /// Általános kártya-klónozó: bemenet az eredeti Card, plusz opcionális power/toughness
    /// felülírás, plusz bitflag hozzáadás.
    pub fn clone_card(
        original: &Card,
        new_power: Option<i32>,
        new_toughness: Option<i32>,
        added_flags: Option<CardTypeFlags>,
    ) -> Card {
        debug!("clone_card called: original id={}, name='{}', new_power={:?}, new_toughness={:?}, added_flags={:?}",
           original.card_id, original.name, new_power, new_toughness, added_flags);

        let mut cloned = original.clone();

        if let CardType::Creature(ref mut cr) = cloned.card_type {
            if let Some(p) = new_power {
                debug!("clone_card: setting new power {} (was {})", p, cr.power);
                cr.power = p;
            }
            if let Some(t) = new_toughness {
                debug!("clone_card: setting new toughness {} (was {})", t, cr.toughness);
                cr.toughness = t;
            }
        }

        if let Some(flags) = added_flags {
            debug!("clone_card: adding flags {:?}", flags);
            cloned.type_flags |= flags;
        }

        // Kiírás a visszaadott másolatról
        let (final_power, final_toughness) = match cloned.card_type {
            CardType::Creature(ref cr) => (cr.power, cr.toughness),
            _ => (0, 0),
        };
        debug!("clone_card returning: cloned id={}, name='{}', power={}, toughness={}, flags={:?}",
           cloned.card_id, cloned.name, final_power, final_toughness, cloned.type_flags);

        cloned
    }
}

/// A teljes card library
pub fn build_card_library() -> HashMap<String, Card> {
    let mut lib = HashMap::new();

    // Cacophony Scamp
    lib.insert(
        CACOPHONY_SCAMP.into(),
        Card::new(
            CACOPHONY_SCAMP,
            CardType::Creature(Creature {
                power: 1,
                toughness: 1,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Phyrexian, CreatureType::Goblin, CreatureType::Warrior],
            }),
            ManaCost::new(0, 1, 0, 0, 0, 0),
        )
            .with(
                Trigger::OnCombatDamage { filter: TargetFilter::SelfCard },
                ProliferateAttribute {
                    counter: CounterType::PlusOnePlusOne,
                    player: PlayerSelector::Controller,
                },
            )
            .with(
                Trigger::OnDeath { filter: TargetFilter::SelfCard },
                TriggeredEffectAttribute {
                    trigger: Trigger::OnDeath { filter: TargetFilter::SelfCard },
                    effect: Effect::Damage {
                        amount: Amount::SourcePower,
                        target: TargetFilter::AnyTarget,
                    },
                },
            ),
    );

    // Monastery Swiftspear
    lib.insert(
        MONASTERY_SWIFTSPEAR.into(),
        Card::new(
            MONASTERY_SWIFTSPEAR,
            CardType::Creature(Creature {
                power: 1,
                toughness: 2,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Human, CreatureType::Monk],
            }),
            ManaCost::new(0, 1, 0, 0, 0, 0),
        )
            .with(
                Trigger::OnEnterBattlefield { filter: TargetFilter::SelfCard },
                GrantAbilityAttribute {
                    ability: KeywordAbility::Haste,
                    duration: Duration::EndOfTurn,
                    target: TargetFilter::SelfCard,
                },
            )
            .with(
                Trigger::OnSpellCast { filter: SpellFilter::InstantOrSorcery },
                ProwessAttribute {
                    filter: SpellFilter::InstantOrSorcery,
                    power: 1,
                    toughness: 1,
                    duration: Duration::EndOfTurn,
                },
            ),
    );

    // Electrostatic Infantry
    lib.insert(
        ELECTROSTATIC_INFANTRY.into(),
        Card::new(
            ELECTROSTATIC_INFANTRY,
            CardType::Creature(Creature {
                power: 1,
                toughness: 2,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Dwarf, CreatureType::Wizard],
            }),
            ManaCost::new(1, 1, 0, 0, 0, 0),
        )
            .with(
                Trigger::OnEnterBattlefield { filter: TargetFilter::SelfCard },
                GrantAbilityAttribute {
                    ability: KeywordAbility::Trample,
                    duration: Duration::Permanent,
                    target: TargetFilter::SelfCard,
                },
            )
            .with(
                Trigger::OnSpellCast { filter: SpellFilter::InstantOrSorcery },
                AddCounterAttribute {
                    counter: CounterType::PlusOnePlusOne,
                    amount: 1,
                    target: TargetFilter::SelfCard,
                },
            ),
    );

    // Heartfire Hero
    lib.insert(
        HEARTFIRE_HERO.into(),
        Card::new(
            HEARTFIRE_HERO,
            CardType::Creature(Creature {
                power: 1,
                toughness: 1,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Mouse, CreatureType::Soldier],
            }),
            ManaCost::new(0, 1, 0, 0, 0, 0),
        )
            .with(
                Trigger::OnTargetedFirstTimeEachTurn { filter: TargetFilter::SelfCard },
                FirstTimePerTurnAttribute {
                    base_trigger: Trigger::OnTargetedFirstTimeEachTurn { filter: TargetFilter::SelfCard },
                    reset_phase: GamePhase::End,
                    action: Effect::AddCounter {
                        counter: CounterType::PlusOnePlusOne,
                        amount: 1,
                        target: TargetFilter::SelfCard,
                    },
                    used: false,
                },
            ),
    );

    // Screaming Nemesis
    lib.insert(
        "Screaming Nemesis".into(),
        Card::new(
            "Screaming Nemesis",
            CardType::Creature(Creature {
                power: 3,
                toughness: 3,
                summoning_sickness: true,
                abilities: vec![KeywordAbility::Haste],
                types: vec![CreatureType::Spirit],
            }),
            ManaCost::new(2, 1, 0, 0, 0, 0),
        )
            .with(
                Trigger::OnEnterBattlefield { filter: TargetFilter::SelfCard },
                GrantAbilityAttribute {
                    ability: KeywordAbility::Haste,
                    duration: Duration::Permanent,
                    target: TargetFilter::SelfCard,
                },
            )
            .with(
                Trigger::OnDealtDamage { filter: TargetFilter::SelfCard },
                TriggeredEffectAttribute {
                    trigger: Trigger::OnDealtDamage { filter: TargetFilter::SelfCard },
                    effect: Effect::Damage {
                        amount: Amount::SourcePower,
                        target: TargetFilter::AnyTarget,
                    },
                },
            )
            .with(
                Trigger::OnDealtDamage { filter: TargetFilter::SelfCard },
                TriggeredEffectAttribute {
                    trigger: Trigger::OnDealtDamage { filter: TargetFilter::SelfCard },
                    effect: Effect::PreventLifeGain {
                        player: PlayerSelector::Opponent,
                        duration: Duration::Permanent,
                    },
                },
            ),
    );

    // Hired Claw
    lib.insert(
        "Hired Claw".into(),
        Card::new(
            "Hired Claw",
            CardType::Creature(Creature {
                power: 1,
                toughness: 2,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Lizard, CreatureType::Mercenary],
            }),
            ManaCost::new(0, 1, 0, 0, 0, 0),
        )
            .with(
                Trigger::OnAttackWithCreatureType { creature_type: CreatureType::Lizard },
                TriggeredEffectAttribute {
                    trigger: Trigger::OnAttackWithCreatureType { creature_type: CreatureType::Lizard },
                    effect: Effect::Damage {
                        amount: Amount::Fixed(1),
                        target: TargetFilter::OpponentCreature,
                    },
                },
            )
            .with_activated(
                ActivatedAbility {
                    cost: ManaCost::new(1, 1, 0, 0, 0, 0),
                    condition: Condition::OpponentLostLifeThisTurn,
                    effect: Effect::AddCounter {
                        counter: CounterType::PlusOnePlusOne,
                        amount: 1,
                        target: TargetFilter::SelfCard,
                    },
                    activated_this_turn: false,
                }
            ),
    );

    // -- FONTOS: Manifold Mouse beépített Offspring 2

    lib.insert(
        "Manifold Mouse".into(),
        Card::new(
            "Manifold Mouse",
            CardType::Creature(Creature {
                power: 1,
                toughness: 2,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: vec![CreatureType::Mouse, CreatureType::Soldier],
            }),
            ManaCost::new(1, 1, 0, 0, 0, 0),
        )
            .with_added_type(CardTypeFlags::CREATURE)

            .with(
                Trigger::AtPhase { phase: GamePhase::BeginningCombat, player: PlayerSelector::Controller },
                ChooseOnConditionAttribute {
                    choose: 1,
                    options: vec![
                        Effect::GrantAbility {
                            ability: KeywordAbility::DoubleStrike,
                            duration: Duration::EndOfTurn,
                            target: TargetFilter::CreatureType(CreatureType::Mouse),
                        },
                        Effect::GrantAbility {
                            ability: KeywordAbility::Trample,
                            duration: Duration::EndOfTurn,
                            target: TargetFilter::CreatureType(CreatureType::Mouse),
                        },
                    ],
                },
            )
            .with(
                Trigger::OnCastResolved,
                ChooseOnConditionAttribute {
                    choose: 1,
                    options: vec![
                        Effect::Damage {
                            amount: Amount::Fixed(0),
                            target: TargetFilter::AnyTarget,
                        },
                        Effect::Offspring { cost: 2 },
                    ],
                },
            ),
    );

    // Slickshot Show-Off
    lib.insert(
        "Slickshot Show-Off".into(),
        Card::new(
            "Slickshot Show-Off",
            CardType::Creature(Creature {
                power: 1,
                toughness: 2,
                summoning_sickness: true,
                abilities: vec![KeywordAbility::Flying, KeywordAbility::Haste],
                types: vec![CreatureType::Bird, CreatureType::Wizard],
            }),
            ManaCost::new(1, 1, 0, 0, 0, 0),
        )
            .with(
                Trigger::OnSpellCast { filter: SpellFilter::InstantOrSorcery },
                BuffAttribute {
                    power: 2,
                    toughness: 0,
                    duration: Duration::EndOfTurn,
                    target: TargetFilter::SelfCard,
                },
            ),
    );

    // Sunset Strikemaster
    lib.insert(
        "Sunset Strikemaster".into(),
        Card::new(
            "Sunset Strikemaster",
            CardType::Creature(Creature {
                power: 3,
                toughness: 1,
                summoning_sickness: true,
                abilities: vec![],
                types: vec![CreatureType::Human, CreatureType::Monk],
            }),
            ManaCost::new(1, 1, 0, 0, 0, 0),
        )
            .with_activated(
                ActivatedAbility {
                    cost: ManaCost::free(),
                    condition: Condition::Always,
                    effect: Effect::AddMana {
                        red: 1,
                        colorless: 0,
                        blue: 0,
                        green: 0,
                        black: 0,
                        white: 0,
                    },
                    activated_this_turn: false,
                }
            ),
    );

    // Emberheart Challenger
    lib.insert(
        "Emberheart Challenger".into(),
        Card::new(
            "Emberheart Challenger",
            CardType::Creature(Creature {
                power: 2,
                toughness: 2,
                summoning_sickness: true,
                abilities: vec![KeywordAbility::Haste],
                types: vec![CreatureType::Mouse, CreatureType::Warrior],
            }),
            ManaCost::new(1, 1, 0, 0, 0, 0),
        )
            .with(
                Trigger::OnSpellCast { filter: SpellFilter::InstantOrSorcery },
                ProwessAttribute {
                    filter: SpellFilter::InstantOrSorcery,
                    power: 1,
                    toughness: 1,
                    duration: Duration::EndOfTurn,
                },
            )
            .with(
                Trigger::OnTargetedFirstTimeEachTurn { filter: TargetFilter::SelfCard },
                FirstTimePerTurnAttribute {
                    base_trigger: Trigger::OnTargetedFirstTimeEachTurn { filter: TargetFilter::SelfCard },
                    reset_phase: GamePhase::End,
                    action: Effect::ExileThenPlayFromExile {
                        count: 1,
                        player: PlayerSelector::Controller,
                        duration: Duration::EndOfTurn,
                    },
                    used: false,
                },
            ),
    );

    // Felonious Rage
    lib.insert(
        FELONIOUS_RAGE.into(),
        Card::new(
            FELONIOUS_RAGE,
            CardType::Instant,
            ManaCost::new(0, 1, 0, 0, 0, 0),
        )
            .with(
                Trigger::OnCastResolved,
                TriggeredEffectAttribute {
                    trigger: Trigger::OnCastResolved,
                    effect: Effect::TargetedEffects {
                        sub_effects: vec![
                            // 1) +2/+0 EoT
                            Effect::ModifyStats {
                                power_delta: 2,
                                toughness_delta: 0,
                                duration: Duration::EndOfTurn,
                                target: TargetFilter::Creature,
                            },
                            // 2) Grant Haste EoT
                            Effect::GrantAbility {
                                ability: KeywordAbility::Haste,
                                duration: Duration::EndOfTurn,
                                target: TargetFilter::Creature,
                            },
                            // 3) Ha meghal a célpont a körben, hozz létre 2/2 nyomozót
                            Effect::WhenTargetDiesThisTurn {
                                effect: Box::new(Effect::CreateCreatureToken {
                                    name: "Detective".into(),
                                    power: 2,
                                    toughness: 2,
                                    creature_types: vec![Detective],
                                }),
                            },
                        ],
                    },
                },
            )
    );

    // Monstrous Rage
    lib.insert(
        MONSTROUS_RAGE.into(),
        Card::new(
            MONSTROUS_RAGE,
            CardType::Instant,
            ManaCost::new(0, 1, 0, 0, 0, 0),
        )
            .with(
                Trigger::OnCastResolved,
                TriggeredEffectAttribute {
                    trigger: Trigger::OnCastResolved,
                    effect: Effect::TargetedEffects {
                        sub_effects: vec![
                            // (1) Eredeti, mondjuk +2/+0 EoT a célpontra
                            Effect::ModifyStats {
                                power_delta: 2,
                                toughness_delta: 0,
                                duration: Duration::EndOfTurn,
                                target: TargetFilter::Creature,
                            },
                            // (2) Létrehozunk egy 'Monster' enchantmentet,
                            //     ami +1/+1‐et és Trample‐t ad
                            //     addig, amíg fennmarad
                            Effect::CreateEnchantmentToken {
                                name: "Monster".into(),
                                power_buff: 1,
                                toughness_buff: 1,
                                ability: KeywordAbility::Trample,
                            },
                        ],
                    },
                },
            )
    );



    // Blazing Crescendo
    lib.insert(
        BLAZING_CRESCENDO.into(),
        Card::new(
            BLAZING_CRESCENDO,
            CardType::Instant,
            ManaCost::new(1, 1, 0, 0, 0, 0),
        )
            .with(
                Trigger::OnCastResolved,
                BuffAttribute {
                    power: 3,
                    toughness: 1,
                    duration: Duration::EndOfTurn,
                    target: TargetFilter::Creature,
                },
            )
            .with(
                Trigger::OnCastResolved,
                ExileAndPlayAttribute {
                    count: 1,
                    player: PlayerSelector::Controller,
                    duration: Duration::NextTurnEnd,
                },
            ),
    );

    // Demonic Ruckus
    lib.insert(
        DEMONIC_RUCKUS.into(),
        Card::new(
            DEMONIC_RUCKUS,
            CardType::Enchantment,
            ManaCost::new(0, 1, 0, 0, 0, 0),
        )
            .with(
                Trigger::OnCastResolved,
                BuffAttribute {
                    power: 1,
                    toughness: 1,
                    duration: Duration::Permanent,
                    target: TargetFilter::ControllerCreature,
                },
            )
            .with(
                Trigger::OnCastResolved,
                GrantAbilityAttribute {
                    ability: KeywordAbility::Menace,
                    duration: Duration::Permanent,
                    target: TargetFilter::ControllerCreature,
                },
            )
            .with(
                Trigger::OnCastResolved,
                GrantAbilityAttribute {
                    ability: KeywordAbility::Trample,
                    duration: Duration::Permanent,
                    target: TargetFilter::ControllerCreature,
                },
            )
            .with(
                Trigger::OnDeath { filter: TargetFilter::SelfCard },
                TriggeredEffectAttribute {
                    trigger: Trigger::OnDeath { filter: TargetFilter::SelfCard },
                    effect: Effect::DrawCards { count: 1, player: PlayerSelector::Controller },
                },
            ),
    );

    // Burst Lightning
    lib.insert(
        BURST_LIGHTNING.into(),
        Card::new(
            BURST_LIGHTNING,
            CardType::Instant,
            ManaCost::new(4, 1, 0, 0, 0, 0),
        )
            .with(
                Trigger::OnCastResolved,
                ConditionalAttribute {
                    trigger: Trigger::OnCastResolved,
                    condition: Condition::SpellWasKicked,
                    effect_if_true: Effect::Damage {
                        amount: Amount::Fixed(4),
                        target: TargetFilter::AnyTarget,
                    },
                    effect_if_false: Some(Effect::Damage {
                        amount: Amount::Fixed(2),
                        target: TargetFilter::AnyTarget,
                    }),
                },
            ),
    );

    // Lightning Strike
    lib.insert(
        LIGHTNING_STRIKE.into(),
        Card::new(
            LIGHTNING_STRIKE,
            CardType::Instant,
            ManaCost::new(0, 1, 0, 0, 0, 0),
        )
            .with(
                Trigger::OnCastResolved,
                TriggeredEffectAttribute {
                    trigger: Trigger::OnCastResolved,
                    effect: Effect::Damage {
                        amount: Amount::Fixed(3),
                        target: TargetFilter::AnyTarget,
                    },
                },
            ),
    );

    // Basic lands
    lib.insert(
        MOUNTAIN.into(),
        Card::new(MOUNTAIN, CardType::Land, ManaCost::free()),
    );
    lib.insert(
        ROCKFACE_VILLAGE.into(),
        Card::new(ROCKFACE_VILLAGE, CardType::Land, ManaCost::free()),
    );

    lib
}
