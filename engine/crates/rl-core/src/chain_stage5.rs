//! Register-preserving owner entries for the live chain past construction.
use crate::{EngineError, FrameRegisters, MemoriaJogo, Result};
pub fn entry(m: &mut MemoriaJogo, name: &str, r: FrameRegisters) -> Result<FrameRegisters> {
    match name {
        "UnblockFreeNeighbor:AX" | "EnginDebloqueNextCase:AL" | "DeplaceEngin:effects" => {
            let value = match name {
                "UnblockFreeNeighbor:AX" => u32::from(crate::chain_unblock::free_neighbor(
                    m,
                    r.eax as u16,
                    r.edx as u16,
                )?),
                "EnginDebloqueNextCase:AL" => {
                    u32::from(crate::chain_unblock::unblock(m, r.eax as u16)?)
                }
                _ => crate::frame_movement::displace(m, r.eax & 65535)?,
            };
            Ok(FrameRegisters {
                eax: value,
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "RestoreAllLoad:effects" => {
            crate::frame_input::display_effects(m, 2)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "CargoFind:AX" | "BagRetireIndice:AX" => {
            let value = if name == "CargoFind:AX" {
                crate::chain_cargo::find(m, r.eax as u16, r.edx as u8)?
            } else {
                crate::chain_cargo::remove_index(m, r.eax, r.edx as u16)?
            };
            Ok(FrameRegisters {
                eax: u32::from(value),
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "CargoDrop:effects" | "SacDepot:effects" => {
            if name == "CargoDrop:effects" {
                crate::chain_cargo::drop_slot(m, r.eax as u16, r.edx as u16)?;
            } else {
                crate::chain_cargo::deposit(m, r.eax as u16, r.edx as u8)?;
            }
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "EnginNextEtape:arrival:effects" => {
            crate::frame_units::arrival(m, r.edi as u16)?;
            Ok(r)
        }
        "EnginNextEtape:15:effects" => {
            crate::chain_repair::state15(m, r.edi as u16)?;
            Ok(r)
        }
        "EnginPeutPasser:search:AL" => {
            // chain_search's EnginPeutPasser 0x442668 as its callers use it:
            // BL = 1 and no occupant array (ECX = 0).
            let ok = crate::chain_search::passable(m, r.eax & 65535, r.edx & 65535)?;
            Ok(FrameRegisters {
                eax: (r.eax & !255) | u32::from(ok),
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "BatimentGenereFumee:effects" => {
            crate::building_dismantle::smoke(m, r.eax as u16)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "CalculRepareEngin:effects" => {
            crate::chain_repair::calculate(m, r.eax & 65535)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "Chemin:allied:AL" => {
            let allied = crate::frame_path::allied(m, r.eax as u8, r.edx as u8)?;
            Ok(FrameRegisters {
                eax: u32::from(allied),
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "AvanceEngin:effects" => {
            crate::frame_movement::advance(m, r.eax & 65535, r.edx as u8)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "CalculFatigue:effects" => {
            crate::frame_core::fatigue(m, r.eax)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "EnginNextEtape:auto:effects" => {
            crate::frame_units::automatic_attack(m, r.edi as u16, r.esi, r.edx as u8, r.ebx as u8)?;
            Ok(r)
        }
        "SetDyingEngin:effects"
        | "EnginSetLevelExperience:effects"
        | "EnginNextEtape:4:effects" => {
            match name {
                "SetDyingEngin:effects" => crate::chain_death::set(m, r.eax as u16)?,
                "EnginSetLevelExperience:effects" => {
                    crate::chain_death::level(m, r.eax as u16, r.edx as u8)?
                }
                _ => crate::chain_death::state4(m, r.edi as u16)?,
            }
            Ok(FrameRegisters {
                esp: if name == "EnginNextEtape:4:effects" {
                    r.esp
                } else {
                    r.esp.wrapping_add(4)
                },
                ..r
            })
        }
        "MessageBoxSet:effects" | "PlayerResetAllie:effects" => {
            if name == "MessageBoxSet:effects" {
                crate::message_box::set(m, r.eax as u8, r.edx as u16)?;
            } else {
                crate::message_box::reset_allie(m, r.eax as u8, r.edx as u8)?;
            }
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "CalculAttaqueEngin:effects" => {
            crate::chain_attack::approach(m, r.eax as u16)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "BatimentNextEtape:population:effects" => {
            crate::frame_units::call(m, "BatimentNextEtape", &[r.eax])?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "iagUnitSendMessage:effects" => {
            crate::frame_ai::call(m, "iagUnitSendMessage", &[r.eax, r.edx, r.ebx])?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "ProjectileAvance:effects" => {
            crate::frame_combat::advance(m, r.eax)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "TextGetMessage" => crate::text::get_registers(m, r),
        "EnginNextEtape:3:effects" | "EnginNextEtape:6:effects" => {
            match name {
                "EnginNextEtape:3:effects" => crate::unit_park::state3(m, r.edi as u16)?,
                _ => crate::unit_park::state6(m, r.edi as u16)?,
            }
            Ok(r)
        }
        "CalculAttaqueEngin:3:effects" => {
            crate::ai_building_fire_chain::approach(m, r.eax as u16)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "EnginNextEtape:23:effects" => {
            crate::player_spy::state23(m, r.edi as u16)?;
            Ok(r)
        }
        "EnginNextEtape:16:effects" => {
            crate::chain_work::pickup(m, r.edi & 65535)?;
            Ok(r)
        }
        "StockeAction:effects" => {
            let value =
                crate::frame_raid::stock_action(m, r.eax, r.edx, r.ebx as u16, r.ecx as u16)?;
            Ok(FrameRegisters {
                eax: (r.eax & !255) | (value & 255),
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "IconeScroll" => {
            crate::player_input::icon_scroll(m, r.eax, r.edx)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "SacGetCapacite" | "TuyauEstSortie" | "MineGetCapacite" | "iagVilleSendMessage" => {
            crate::chain_queries::entry(m, name, r)
        }
        "CalculTransporteEngin:effects" => {
            crate::transport_delivery_next::transport(m, r.eax as u16)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "GetCaseCentrale" => Ok(FrameRegisters {
            eax: (r.edi & 0xffff0000).wrapping_add(crate::chain_entry::central_offset(m, r.eax)?),
            esp: r.esp.wrapping_add(4),
            ..r
        }),
        "EnginSetType"
        | "EnginAddVie"
        | "EnginResetFatigue"
        | "EnginSetExperimente"
        | "SetSortEngin" => crate::chain_recruit::entry(m, name, r),
        "EnginSelectPeutEntrer:AL" => {
            let ok = crate::chain_entry::can_enter(m, r.eax & 255, r.edx & 255, r.ebx & 65535)?;
            Ok(FrameRegisters {
                eax: u32::from(ok),
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "BatimentAjouteMatiere:effects" => {
            crate::chain_entry::credit(m, r.eax as u8, r.edx as u16, r.ebx as u8, r.ecx as u16)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "SetMiniMap:effects" => {
            crate::player_sight::minimap(m, r.eax as u8, r.ebx as u16, r.edx as u8)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "CalculTroupe:effects" => {
            crate::frame_economy::production(m, r.eax & 65535, false, true)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "TerrainEstGc"
        | "ChercheVeget"
        | "ChercheGc"
        | "ChercheGcLibre"
        | "TypeEnginPeutEcraser"
        | "IsDirectlyBlocking"
        | "IsIndirectlyBlocking" => {
            let mut q = crate::chain_search::entry(m, name, r)?;
            q.esp = q.esp.wrapping_add(4);
            Ok(q)
        }
        "BatimentPeutAttaquer" | "BatimentInitAttaqueSelect" => {
            let mut q = crate::chain_turret::entry(m, name, r)?;
            q.esp = q.esp.wrapping_add(4);
            Ok(q)
        }
        "ChercheBatimentProximiteCaveOuSanctuaire"
        | "TerrainEstMontagne"
        | "EnginEstVivant"
        | "BatimentGetDecalageConstruction"
        | "GetCaseIndiceBatiment"
        | "ChercheCaseBatiment"
        | "BatimentGetCaseAdjacent"
        | "listDeleteElem"
        | "listUnlinkElem"
        | "iaProbReturnTo"
        | "VoixOffAsk" => {
            let mut q = crate::chain_s1::entry(m, name, r)?;
            q.esp = q.esp.wrapping_add(4);
            Ok(q)
        }
        "MineGetIndice" | "MineEstSurTerrain" | "MineSetWork" | "MineEndWork" | "MineRecolte"
        | "TerrainRecolte" | "AjouteMatiere" => {
            let mut q = crate::chain_work::leaf(m, name, r)?;
            q.esp = q.esp.wrapping_add(4);
            Ok(q)
        }
        "CalculWorkEngin:effects"
        | "EnginRestaureEtat:effects"
        | "BatimentEnleveMatiere:effects"
        | "SetDechargeCaveEngin:effects"
        | "SetDechargeCaveEnginSelect:effects"
        | "EnginSetFatigue:effects"
        | "InitRecruteTroupe:effects"
        | "SortieTroupe:effects"
        | "BatimentRetireColon:effects" => {
            match name {
                "CalculWorkEngin:effects" => {
                    crate::chain_work::work(m, r.eax & 65535)?;
                }
                "EnginRestaureEtat:effects" => crate::chain_restore::restore(m, r.eax & 65535)?,
                "BatimentEnleveMatiere:effects" => {
                    crate::chain_restore::debit(m, r.eax as u8, r.edx as u16, r.ebx as u8)?
                }
                "SetDechargeCaveEngin:effects" => {
                    crate::chain_restore::unload(m, r.eax & 65535, r.edx & 65535)?
                }
                "SetDechargeCaveEnginSelect:effects" => {
                    crate::chain_restore::select_unload(m, r.eax & 255, r.edx & 65535)?
                }
                "EnginSetFatigue:effects" => crate::chain_fatigue::set(m, r.eax & 65535)?,
                "InitRecruteTroupe:effects" => {
                    crate::chain_recruit::init(m, r.eax & 255, r.edx & 255)?;
                }
                "SortieTroupe:effects" => {
                    crate::chain_recruit::sortie(m, r.eax & 65535, r.edx & 255)?
                }
                _ => {
                    crate::chain_recruit::retire_colonist(m, r.eax & 65535)?;
                }
            }
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "TerrainRetireMatiere" => {
            let eax = crate::chain_restore::retire_food(m, r.eax & 65535)?;
            Ok(FrameRegisters {
                eax,
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "EnginNextEtape:19:effects" => {
            crate::chain_restore::hunger(m, r.edi & 65535)?;
            Ok(r)
        }
        "InitBatimentPlayer:effects" => {
            crate::chain_habitation::own(m, "InitBatimentPlayer", r)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "RotateEngin:effects" => {
            crate::frame_movement::call(m, "RotateEngin", &[r.eax & 65535])?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "PlaceEngin:effects" => {
            crate::frame_spawn::call(m, "PlaceEngin", &[r.eax & 65535])?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "EnginNextEtape:13:effects" => {
            let u = (r.edi & 65535) as u16;
            let rec = crate::player::unit(m, u32::from(u))?;
            crate::chain_entry::state13(m, u, rec)?;
            Ok(r)
        }
        "EnginNextEtape:20:effects" => {
            crate::transport_delivery_next::state20(m, (r.edi & 65535) as u16)?;
            Ok(r)
        }
        "CalculDechargeCaveEngin:effects" => {
            crate::transport_delivery_next::cave_unload(m, (r.eax & 65535) as u16)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "EnginNextEtape:5:effects" => {
            crate::frame_units::work_state(m, (r.edi & 65535) as u16)?;
            Ok(r)
        }
        "EnginNextEtape:17:effects" => {
            crate::frame_units::balloon_step(m, (r.edi & 65535) as u16)?;
            Ok(r)
        }
        "EnginNextEtape:rotation-gate:effects" => {
            crate::stage_unit_progression::before_state(m, (r.edi & 65535) as u16)?;
            Ok(r)
        }
        "AnimBallonStatic:effects" => {
            crate::frame_movement::balloon_animation(m, r.eax & 65535)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "BatimentNextEtape:dispatch:effects" => {
            crate::frame_units::call(m, "BatimentNextEtape", &[r.eax & 65535])?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "BatimentNextEtape:cave" | "BatimentNextEtape" => crate::player_cave::entry(m, r),
        "BatimentIllumine" | "BatimentDecouvre" | "IllumineCase" | "EteintCase"
        | "DecouvreCase" => {
            let mut r = crate::chain_vision::entry(m, name, r)?;
            r.esp = r.esp.wrapping_add(4);
            Ok(r)
        }
        "SauveSelectionGroupe" | "RestaureSelectionGroupe" => {
            let mut r = crate::chain_groups::entry(m, name, r)?;
            r.esp = r.esp.wrapping_add(4);
            Ok(r)
        }
        "MiniMapSetPos" | "MiniMapRecentre" | "CalculPosition" | "RechercheDecalageSortie" => {
            let mut r = crate::chain_vision::extra(m, name, r)?;
            r.esp = r.esp.wrapping_add(4);
            Ok(r)
        }
        "EnginGetCentreGroupe" => {
            let mut r = crate::chain_groups::centre(m, r)?;
            r.esp = r.esp.wrapping_add(4);
            Ok(r)
        }
        "BagContient" | "BagContientTroupe" | "BagPrend" | "BagRetire" | "BagGetNbContenu"
        | "BagGetCode" | "BagGetType" | "BagAccepte" | "EnginGetBag" => {
            let mut out = crate::chain_bags::body(m, name, r)?;
            out.esp = out.esp.wrapping_add(4);
            Ok(out)
        }
        "BatimentHabitable"
        | "BatimentSetTechnologie"
        | "InitBatimentPlayer"
        | "InitBatimentNeutre"
        | "EnginGetTimeRecrute"
        | "EnginGetRessource"
        | "TypeEnginEstRealisable"
        | "BatimentGetCapacite" => crate::chain_habitation::own(m, name, r),
        "PlaceLibre" => {
            let mut out = crate::chain_entry::place_free(m, r)?;
            out.esp = out.esp.wrapping_add(4);
            Ok(out)
        }
        "BatimentAdjacent:AL" | "BatimentLogeEngin:AL" => {
            let value = if name == "BatimentAdjacent:AL" {
                crate::chain_entry::adjacent(m, r.eax as u16, r.edx as u16)?
            } else {
                crate::chain_entry::lodge(m, r.eax as u16, r.edx as u16)?
            };
            Ok(FrameRegisters {
                eax: u32::from(value),
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "SetEnginRentreBatiment:effects"
        | "SetEnginSelectRentreBatiment:effects"
        | "CalculEntreBatiment:effects"
        | "RetireEngin:effects"
        | "EnginResetType:effects" => {
            match name {
                "SetEnginRentreBatiment:effects" => {
                    crate::chain_entry::set_enter(m, r.eax as u16, r.edx as u16)?
                }
                "SetEnginSelectRentreBatiment:effects" => {
                    crate::chain_entry::select_enter(m, r.eax as u8, r.edx as u16)?
                }
                "CalculEntreBatiment:effects" => crate::chain_entry::calculate(m, r.eax as u16)?,
                "RetireEngin:effects" => crate::chain_entry::retire(m, r.eax as u16)?,
                _ => crate::chain_entry::reset_type(m, r.eax as u16)?,
            }
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "InfligeDegat:effects" | "EnginInfligeDegat:effects" => {
            crate::chain_damage::fixture(m, r, name == "EnginInfligeDegat:effects")
        }
        "EnginNextEtape:state1:effects" => crate::chain_attack::fixture(m, r),
        "EnginNextEtape:reload:effects" => {
            crate::frame_units::reload(m, r.edi as u16)?;
            Ok(r)
        }
        "EnginNextEtape:state13:effects" => {
            crate::chain_entry::state13(m, r.edi as u16, r.esi)?;
            Ok(r)
        }
        "BatimentModifyMap:effects" => {
            // The owner's contract leaves EAX/EDX scratch unspecified. This
            // test adapter compares effects and the preserved register file.
            crate::chain_vision::structure_minimap(m, r.eax as u16, r.edx as u8)?;
            Ok(FrameRegisters {
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        "RechercheDecalageSortie:AX" => {
            // Explicit u16 semantic return: this adapter makes no claim for
            // the original's uninitialized EAX upper stack word.
            let value = crate::chain_vision::exit_cell(m, r.eax & 65535)?;
            Ok(FrameRegisters {
                eax: u32::from(value),
                esp: r.esp.wrapping_add(4),
                ..r
            })
        }
        _ => Err(EngineError::UnknownRoutine(0)),
    }
}
