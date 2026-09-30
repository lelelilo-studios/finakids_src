//! The story of Sofía: weekly chapters, conversations, decisions and consequences.

use super::finance::{InvestKind, Skill};
use super::items::{self, Shop};
use super::script::*;
use super::time::Phase;
use super::*;
use crate::character::face::Expr;
use crate::character::HeldProp;
use crate::math::money;
use crate::world::Loc;

pub const LAST_WEEK: u32 = 8;

// ------------------------------------------------------------------ title screen

pub fn setup_title(s: &mut State) {
    s.screen = Screen::Title;
    s.goto(Loc::Bedroom, "start");
    s.hour = 19.3;
    s.hour_target = 19.3;
    s.fade = 1.0;
    s.fade_target = 0.0;
    s.letterbox_target = 0.0;
    s.dir = Director::new();
    s.dialog = None;
    s.choice = None;
    s.modal = None;
    s.objective = None;
    let seat = s.locs[s.cur].seat("bed").cloned();
    if let Some(seat) = seat {
        s.chars[0].anim.teleport(Vec3::new(seat.pos.x, 0.0, seat.pos.z + 0.3), seat.yaw);
        s.chars[0].anim.sit(seat.pos, seat.yaw, seat.feet);
    }
    s.chars[0].held[1] = Some(HeldProp::Phone);
    let k = s.chars[0].k();
    let (view, palm) = crate::character::actions::phone_view_pos(k);
    s.chars[0].anim.play(crate::character::anim::Action {
        name: "title_phone",
        hold: true,
        phases: vec![crate::character::anim::ph(
            0.6,
            crate::math::Ease::InOut,
            vec![
                crate::character::anim::Set::Hand(1, view - Vec3::new(0.0, 0.12, 0.05) * k, palm, 1.0),
                crate::character::anim::Set::Grip(1, 0.6),
                crate::character::anim::Set::LookLocal(view - Vec3::new(0.0, 0.1, 0.05) * k),
                crate::character::anim::Set::Expr(Expr::Happy),
            ],
        )],
    });
    let li = s.cur;
    s.locs[li].set_prop_visible("phone_table", false);
    let shot = super::camera::Shot::dolly(Vec3::new(-0.6, 1.25, 1.2), Vec3::new(0.2, 1.1, 0.9), Vec3::new(1.0, 0.95, -1.1), 36.0, 30.0);
    s.cam.play(shot);
}

pub fn update_title(s: &mut State, _dt: f32) {
    let _ = s;
}

pub fn on_char_built(s: &mut State, id: &str) {
    let _ = (s, id);
}

// ------------------------------------------------------------------ new game / resume

fn place_cast(s: &mut State) {
    s.place(Who::Mama, Loc::Home, "mom");
    s.place(Who::Tomas, Loc::Plaza, "tomas");
    s.place(Who::Vale, Loc::Plaza, "vale");
    s.place(Who::Julio, Loc::Plaza, "julio");
    if let Some(c) = s.ch(Who::Vale) {
        c.anim.target_yaw = Some(0.0);
    }
}

pub fn new_game(s: &mut State) {
    s.week = 1;
    s.fin.week = 1;
    s.set_phase(Phase::Morning);
    s.hour = 7.7;
    s.hour_target = 7.7;
    s.goto(Loc::Bedroom, "start");
    place_cast(s);
    s.chars[0].held = [None, None];
    s.chars[0].anim.clear_action();
    let li = s.cur;
    s.locs[li].set_prop_visible("phone_table", true);
    if let Some(seat) = s.locs[li].seat("bed").cloned() {
        s.chars[0].anim.teleport(Vec3::new(seat.pos.x, 0.0, seat.pos.z + 0.35), seat.yaw);
        s.chars[0].anim.sit(seat.pos, seat.yaw, seat.feet);
    }
    s.fade = 1.0;
    s.objective = None;
    s.dir.run(intro());
}

pub fn resume(s: &mut State) {
    place_cast(s);
    s.goto(Loc::Bedroom, "start");
    s.set_phase(Phase::Morning);
    s.hour = Phase::Morning.hour();
    refresh_room(s);
    s.dir.run(vec![
        Step::Fade(false, 1.2),
        Step::Title(format!("Semana {} · Sábado", s.week), chapter_name(s.week).to_string()),
    ]);
    s.objective = Some(default_objective(s));
}

/// Shows owned items in the bedroom and goal notes on the corkboard.
pub fn refresh_room(s: &mut State) {
    let owned: Vec<&'static str> = s.fin.inventory.clone();
    let goals = s.fin.goals.iter().filter(|g| !g.done).count();
    let loc = s.loc_mut(Loc::Bedroom);
    for it in items::catalog() {
        if let Some(prop) = it.prop {
            let has = owned.contains(&it.id);
            if has {
                loc.set_prop_visible(prop, true);
            }
        }
    }
    for (i, n) in ["note_0", "note_1", "note_2", "note_3"].iter().enumerate() {
        loc.set_prop_visible(n, i < goals.max(1));
    }
}

pub fn chapter_name(week: u32) -> &'static str {
    match week {
        1 => "Capítulo 1 — Quince",
        2 => "Capítulo 2 — Presupuesto",
        3 => "Capítulo 3 — Imprevisto",
        4 => "Capítulo 4 — Trabajo",
        5 => "Capítulo 5 — Emprender",
        6 => "Capítulo 6 — Invertir",
        7 => "Capítulo 7 — Presión social",
        8 => "Capítulo 8 — El viaje",
        _ => "Vida libre",
    }
}

fn default_objective(s: &State) -> String {
    match s.week {
        1 => "Explora. Cuando termines el día, duerme en tu cama.".into(),
        _ if s.week > LAST_WEEK => "Vive tu vida: trabaja, ahorra, emprende o invierte.".into(),
        _ => "Haz lo que quieras hoy. Cuando termines, duerme en tu cama.".into(),
    }
}

// ------------------------------------------------------------------ intro

fn intro() -> Vec<Step> {
    vec![
        Step::Letterbox(true),
        shot(ShotKind::Wide),
        Step::Fade(false, 2.0),
        Step::Title("FINAKIDS".into(), "Tu vida. Tu dinero. Tus decisiones.".into()),
        Step::Title("Semana 1 · Sábado".into(), "Capítulo 1 — Quince".into()),
        shot(ShotKind::PushIn(Who::Sofia)),
        Step::Expr(Who::Sofia, Expr::Tired),
        act(Who::Sofia, Act::Stretch),
        think("Sábado. Sin despertador... y hoy cumplo quince."),
        Step::Toast("phone", "Bzzz... bzzz".into(), "Tu teléfono vibra sobre el velador".into(), 0),
        think("¿Quién me escribe tan temprano?"),
        Step::Stand(Who::Sofia),
        wait(0.8),
        walk(Who::Sofia, Target::Interact("phone")),
        face(Who::Sofia, Target::Pos(Vec3::new(1.94, 0.6, -0.86))),
        shot(ShotKind::Object("phone")),
        wait(0.4),
        act(Who::Sofia, Act::PickPhone),
        shot(ShotKind::Phone(Who::Sofia)),
        msg("abuela", "¡Feliz cumpleaños, mi niña linda! Te transferí $30.000. Úsalos como tú decidas, ya estás grande. Te quiero mucho."),
        wait(1.2),
        doit(|s| {
            s.fin.earn(30_000, "Regalo de cumpleaños (Abuela Rosa)");
            s.toast("bank", "Transferencia recibida", "Abuela Rosa te envió dinero", 30_000);
        }),
        wait(1.0),
        Step::Expr(Who::Sofia, Expr::Joy),
        think("¡Treinta mil pesos! Nunca había tenido tanta plata junta."),
        Step::Expr(Who::Sofia, Expr::Thinking),
        think("Podría comprarme los audífonos que vi... o juntar para la bici... o..."),
        think("Primero le cuento a mamá."),
        act(Who::Sofia, Act::PhoneAway),
        Step::Expr(Who::Sofia, Expr::Happy),
        objective("Habla con mamá (está en la cocina)"),
        doit(|s| {
            s.hint("Camina con WASD o flechas, o toca el suelo. Acércate a la puerta y presiona E (o tócala).");
            s.set_flag("intro_done");
        }),
    ]
}

// ------------------------------------------------------------------ transitions

fn door(to: Loc, spawn: &'static str) -> Vec<Step> {
    vec![
        Step::Fade(true, 0.45),
        Step::Goto(to, spawn),
        doit(move |s| on_enter(s, to)),
        Step::Fade(false, 0.5),
    ]
}

fn on_enter(s: &mut State, loc: Loc) {
    refresh_room(s);
    match loc {
        Loc::Plaza => {
            if s.week == 1 && !s.flag("w1_plaza") {
                s.set_flag("w1_plaza");
                s.set_phase(Phase::Afternoon);
                s.hour = Phase::Afternoon.hour();
                s.dir.append(vec![
                    Step::Letterbox(true),
                    shot(ShotKind::Wide),
                    Step::Title("Esa tarde...".into(), "Plaza del barrio".into()),
                    act_nb(Who::Tomas, Act::Wave),
                    doit(|s| {
                        s.objective = Some("Habla con Tomás (junto a la banca)".into());
                        s.hint("Toca a un personaje o acércate y presiona E para hablar.");
                    }),
                ]);
            } else if s.phase == Phase::Morning {
                s.set_phase(Phase::Afternoon);
            }
        }
        Loc::Home => {
            if s.week == 1 && !s.flag("w1_mom") {
                s.objective = Some("Habla con mamá".into());
            }
            if s.flag("met_all") && s.phase == Phase::Afternoon && s.flag("left_plaza") {
                s.set_phase(Phase::Evening);
            }
        }
        Loc::Bedroom => {}
    }
    if loc != Loc::Plaza {
        s.set_flag("left_plaza");
    }
}

// ------------------------------------------------------------------ object interactions

pub fn interact(s: &mut State, id: &str) -> Option<Vec<Step>> {
    let steps = match id {
        "door" => door(Loc::Home, "hall"),
        "hall_door" => door(Loc::Bedroom, "door"),
        "front_door" => {
            if s.week == 1 && !s.flag("w1_mom") {
                vec![think("Primero quiero hablar con mamá.")]
            } else {
                door(Loc::Plaza, "home")
            }
        }
        "home" => door(Loc::Home, "front"),
        "phone" => vec![
            act(Who::Sofia, Act::PickPhone),
            Step::Modal(Modal::Phone(PhoneApp::Home)),
            doit(|s| {
                let k = s.chars[0].k();
                let target = s.loc().interactable("phone").map(|i| i.pos).unwrap_or(Vec3::ZERO);
                let local = s.chars[0].anim.to_local(target);
                s.chars[0].anim.play(crate::character::actions::put_down(k, local));
                s.set_flag("phone_on_table");
                let li = s.cur;
                s.locs[li].set_prop_visible("phone_table", true);
            }),
        ],
        "laptop" => laptop(s),
        "piggy" | "corkboard" => vec![Step::Modal(Modal::Phone(PhoneApp::Goals))],
        "bed" => bed(s),
        "window" => {
            let t = match s.phase {
                Phase::Morning => "El sol entra por la ventana. Se escucha a los vecinos regando.",
                Phase::Afternoon => "Tarde tranquila. Alguien pasa en bicicleta por la calle.",
                Phase::Evening => "El cielo se pone naranjo. Me encantan los atardeceres.",
                Phase::Night => "Las luces del barrio se encienden una a una.",
            };
            vec![think(t)]
        }
        "lamp" => {
            let loc = s.loc_mut(Loc::Bedroom);
            if let Some(l) = loc.lamp_mut("night_lamp") {
                l.on = !l.on;
                l.auto_night = false;
            }
            return None;
        }
        "wardrobe" => vec![think(if s.fin.inventory.contains(&"sneakers") {
            "Mis zapatillas nuevas... ya no me emocionan tanto como el primer día."
        } else {
            "Mi ropa de siempre. No está mal."
        })],
        "fridge" => vec![think("Hay almuerzo guardado. Comer en casa es gratis... bueno, lo paga mamá.")],
        "tv" => vec![think("Dan un comercial de tarjetas de crédito: \"¡Compra hoy, paga después!\" Mmm.")],
        "store" => store(s),
        "cafe" => cafe(s),
        "bank" => bank(s),
        "stall" => stall(s),
        "fountain" => fountain(s),
        "bench" => vec![
            Step::Sit(Who::Sofia, "bench_0"),
            wait(1.0),
            think(bench_thought(s)),
            Step::Stand(Who::Sofia),
        ],
        _ => return None,
    };
    Some(steps)
}

fn bench_thought(s: &State) -> &'static str {
    if s.fin.total_debt() > 0 {
        "Todavía debo cuotas... Cada semana se van solas de mi billetera."
    } else if s.fin.savings + s.fin.goals_saved() > 40_000 {
        "Es raro: tener ahorros me da más tranquilidad que comprar cosas."
    } else {
        "La plaza está llena de gente. Todos parecen tener algo que comprar."
    }
}

fn fountain(s: &mut State) -> Vec<Step> {
    if s.fin.wallet < 100 {
        return vec![think("Una fuente de deseos. No tengo ni una moneda.")];
    }
    vec![choice(
        "La fuente de los deseos. ¿Tirar una moneda de $100?",
        vec![
            opt(
                "Tirar la moneda y pedir un deseo",
                vec![
                    doit(|s| {
                        s.fin.spend(100, "Moneda a la fuente");
                        let p = s.chars[0].anim.pos + Vec3::new(0.0, 0.8, -0.8);
                        s.burst(0, p);
                    }),
                    think("Deseo que me alcance para todo lo que quiero... aunque creo que eso depende más de mí que de la fuente."),
                ],
            ),
            opt("Mejor no", vec![think("Cien pesos son cien pesos.")]),
        ],
    )]
}

fn laptop(s: &mut State) -> Vec<Step> {
    let mut v = vec![Step::Sit(Who::Sofia, "desk_chair"), wait(0.9), act_nb(Who::Sofia, Act::TypeLaptop)];
    v.push(doit(|s| {
        let loc = s.loc_mut(Loc::Bedroom);
        if let Some(l) = loc.lamp_mut("laptop_glow") {
            l.on = true;
        }
    }));
    let auto = s.fin.auto_save_pct;
    v.push(choice(
        "Planificador de presupuesto (computador)",
        vec![
            opt_d(
                "Definir una regla de ahorro automática",
                &format!("Actual: {}% de tu mesada", auto),
                vec![choice(
                    "¿Qué porcentaje de tu mesada semanal ($10.000) quieres ahorrar automáticamente?",
                    vec![
                        opt_d("10% ($1.000 por semana)", "Un comienzo suave", vec![doit(|s| set_rule(s, 10))]),
                        opt_d("20% ($2.000 por semana)", "La regla clásica 50/30/20", vec![doit(|s| set_rule(s, 20))]),
                        opt_d("30% ($3.000 por semana)", "Ambiciosa: menos gustos", vec![doit(|s| set_rule(s, 30))]),
                        opt_d("Sin regla", "Ahorraré lo que sobre", vec![doit(|s| set_rule(s, 0))]),
                    ],
                )],
            ),
            opt_d("Revisar cuánto gasto por semana", "Mesada vs. gastos fijos", vec![
                text("Mesada: $10.000  ·  Colaciones: -$4.000  ·  Micro: -$2.000  →  Te quedan $4.000 libres por semana."),
                think("Cuatro mil libres... si me compro un latte de $2.900, casi se me va todo."),
                doit(|s| s.set_flag("checked_budget")),
            ]),
            opt("Ver videos (no hacer nada)", vec![think("Un video más... y otro... y otro.")]),
        ],
    ));
    v.push(act(Who::Sofia, Act::Release));
    v.push(doit(|s| {
        let loc = s.loc_mut(Loc::Bedroom);
        if let Some(l) = loc.lamp_mut("laptop_glow") {
            l.on = false;
        }
    }));
    v.push(Step::Stand(Who::Sofia));
    v
}

fn set_rule(s: &mut State, pct: u32) {
    s.fin.auto_save_pct = pct;
    if pct > 0 {
        s.toast("piggy", "Regla de ahorro activada", &format!("Cada semana se ahorrará el {pct}% de tu mesada"), 0);
        if !s.flag("rule_journal") {
            s.set_flag("rule_journal");
            s.fin.journal(
                "Ahorro automático",
                &format!("Decidiste ahorrar el {pct}% de tu mesada cada semana, antes de gastar."),
                Skill::Planning,
                1,
            );
        }
    } else {
        s.toast("piggy", "Sin regla de ahorro", "Ahorrarás solo lo que sobre", 0);
    }
}

fn bed(s: &mut State) -> Vec<Step> {
    if s.week == 1 && !s.flag("w1_mom") {
        return vec![think("Todavía no. Quiero contarle a mamá lo de la abuela.")];
    }
    let early = matches!(s.phase, Phase::Morning) && s.week > 1 && !s.flag("week_beat_done");
    let mut opts = vec![opt_d(
        "Dormir y terminar el día",
        "Pasará la semana y recibirás tu mesada",
        vec![Step::Dyn(Box::new(|_s| sleep()))],
    )];
    if early {
        opts[0].detail = Some("Aún no has hecho lo más importante de hoy".into());
    }
    opts.push(opt("Todavía no", vec![]));
    vec![Step::Sit(Who::Sofia, "bed"), wait(0.6), choice("¿Terminar el día?", opts), Step::Stand(Who::Sofia)]
}

pub fn sleep() -> Vec<Step> {
    vec![
        Step::Letterbox(true),
        doit(|s| {
            s.set_phase(Phase::Night);
        }),
        wait(1.2),
        Step::Fade(true, 1.2),
        doit(|s| {
            let crash = s.week + 1 == LAST_WEEK;
            let rep = end_week(s, crash);
            s.modal = Some(Modal::Summary(rep));
        }),
        Step::Dyn(Box::new(|_| vec![])),
        doit(|s| {
            s.dir.waiting = Waiting::Modal;
        }),
        Step::Dyn(Box::new(|s| week_start(s))),
    ]
}

fn end_week(s: &mut State, crash: bool) -> WeekReport {
    let mut rep = s.fin.end_week(crash);
    s.week = s.fin.week;
    // cheap headphones break
    if s.fin.inventory.contains(&"headphones_cheap") && !s.flag("cheap_broke") && s.week >= 3 {
        s.set_flag("cheap_broke");
        s.fin.inventory.retain(|i| *i != "headphones_cheap");
        s.loc_mut(Loc::Bedroom).set_prop_visible("item_headphones", false);
        rep.notes.push("Los audífonos genéricos dejaron de funcionar. Sin garantía, no hay reembolso.".into());
        s.fin.journal(
            "Lo barato sale caro",
            "Los audífonos genéricos duraron dos semanas. A veces comparar precio no basta: también importa la calidad y la garantía.",
            Skill::Comparing,
            -1,
        );
    }
    if s.flag("job") {
        rep.notes.push("Este sábado puedes trabajar un turno en Café Aroma.".into());
    }
    if s.biz.active {
        rep.notes.push(format!("Tu negocio tiene {} pulseras en inventario.", s.biz.inventory));
    }
    for f in ["week_beat_done", "worked_today", "sold_today", "cafe_today", "left_plaza"] {
        s.flags.remove(f);
    }
    s.set_phase(Phase::Morning);
    s.hour = Phase::Morning.hour();
    s.hour_target = s.hour;
    s.goto(Loc::Bedroom, "start");
    if let Some(seat) = s.loc().seat("bed").cloned() {
        s.chars[0].anim.teleport(Vec3::new(seat.pos.x, 0.0, seat.pos.z + 0.35), seat.yaw);
        s.chars[0].anim.sit(seat.pos, seat.yaw, seat.feet);
    }
    s.chars[0].held = [None, None];
    place_cast(s);
    refresh_room(s);
    super::save::save(s);
    rep
}

// ------------------------------------------------------------------ weekly beats

pub fn week_start(s: &State) -> Vec<Step> {
    let w = s.week;
    let mut v = vec![
        Step::Fade(false, 1.2),
        Step::Title(format!("Semana {w} · Sábado"), chapter_name(w).to_string()),
        shot(ShotKind::PushIn(Who::Sofia)),
    ];
    match w {
        2 => v.extend(week2()),
        3 => v.extend(week3(s)),
        4 => v.extend(week4()),
        5 => v.extend(week5()),
        6 => v.extend(week6()),
        7 => v.extend(week7()),
        8 => v.extend(week8(s)),
        _ => v.extend(free_week(s)),
    }
    v.push(doit(|s| {
        if s.objective.is_none() {
            s.objective = Some(default_objective(s));
        }
    }));
    v
}

fn week2() -> Vec<Step> {
    vec![
        act(Who::Sofia, Act::Stretch),
        think("Ya pasó una semana. Me llegó la mesada... y ya gasté en colaciones y micro."),
        msg("mama", "Sofi, te recuerdo: el viaje de estudios es en 7 semanas y cuesta $120.000. Yo pongo la mitad. ¿Te animas a juntar los otros $60.000?"),
        wait(1.0),
        Step::Expr(Who::Sofia, Expr::Worried),
        think("¿Sesenta mil en siete semanas? Con cuatro mil libres a la semana no me alcanza..."),
        choice(
            "¿Qué haces con el viaje de estudios?",
            vec![
                opt(
                    "Crear una meta: \"Viaje de estudios\" ($60.000)",
                    vec![
                        doit(|s| {
                            create_goal(s, "trip");
                            s.set_flag("trip_goal");
                        }),
                        Step::Expr(Who::Sofia, Expr::Focused),
                        think("Si lo pongo como meta, voy a ver cuánto me falta cada semana."),
                    ],
                ),
                opt(
                    "Pensarlo después",
                    vec![think("Todavía queda harto tiempo. Ya veré."), doit(|s| s.set_flag("trip_later"))],
                ),
            ],
        ),
        think("Quizás debería hacer un presupuesto en el computador."),
        doit(|s| {
            s.objective = Some("Opcional: planifica tu presupuesto en el computador".into());
            s.set_flag("week_beat_done");
            s.set_flag("flash_sale");
        }),
        msg("tecno", "¡SOLO HOY! Parlante bluetooth a $12.990 (antes $24.990). Stock limitado. TecnoMundo."),
    ]
}

fn week3(s: &State) -> Vec<Step> {
    let has_fund = s.fin.savings + s.fin.goals_saved() >= 25_000;
    let _ = has_fund;
    vec![
        Step::Stand(Who::Sofia),
        wait(0.6),
        walk(Who::Sofia, Target::Interact("phone")),
        act(Who::Sofia, Act::PickPhone),
        think("A ver los mensajes..."),
        act(Who::Sofia, Act::DropPhone),
        doit(|s| {
            s.set_flag("phone_cracked");
            for c in s.chars.iter_mut().filter(|c| c.id == "sofia") {
                c.phone_cracked = 1.0;
            }
        }),
        Step::Expr(Who::Sofia, Expr::Surprised),
        shot(ShotKind::Close(Who::Sofia)),
        say_e(Who::Sofia, Expr::Worried, "¡No, no, no! ¡La pantalla!"),
        doit(|s| s.chars[0].held[1] = Some(HeldProp::Phone)),
        think("Se trizó entera. Todavía funciona, pero casi no se ve nada..."),
        msg("tecno", "Reparación de pantalla: $24.990 en TecnoMundo. También en 3 cuotas de $9.490 con tu Tarjeta TecnoMundo."),
        wait(0.8),
        Step::Dyn(Box::new(|s| {
            let emergency = s.fin.savings + s.fin.goals_saved();
            let mut opts = vec![];
            if emergency >= 24_990 || s.fin.wallet >= 24_990 {
                opts.push(opt_d(
                    "Pagar la reparación con mis ahorros",
                    &format!("Tienes {} entre billetera, ahorro y metas", money(s.fin.wallet + emergency)),
                    vec![doit(|s| repair_with_savings(s))],
                ));
            } else {
                opts.push(opt_locked("Pagar la reparación con mis ahorros", "No tienes suficiente ahorrado"));
            }
            opts.push(opt_d(
                "Pagar en 3 cuotas con la tarjeta",
                "3 cuotas de $9.490 = $28.470 en total",
                vec![doit(|s| {
                    s.fin.take_credit("Reparación de pantalla", 24_990, 3, 0.0 + 0.0666, "tecno");
                    s.set_flag("repaired");
                    s.set_flag("repair_credit");
                    fix_phone(s);
                    s.fin.journal(
                        "Imprevisto pagado a crédito",
                        "Sin ahorros para emergencias, pagaste la pantalla en cuotas. Terminarás pagando $28.470 por una reparación de $24.990.",
                        Skill::Resilience,
                        -1,
                    );
                })],
            ));
            opts.push(opt_d(
                "Pedirle un préstamo a mamá",
                "Sin interés, pero le devuelves $5.000 por semana",
                vec![Step::Dyn(Box::new(|_| ask_mom_loan()))],
            ));
            opts.push(opt_d(
                "Seguir con la pantalla trizada",
                "Gratis... pero incómodo",
                vec![doit(|s| {
                    s.set_flag("cracked_kept");
                    s.fin.mood -= 0.15;
                    s.fin.journal(
                        "Vivir con el problema",
                        "Decidiste no reparar el teléfono. No gastaste, pero cada vez que lo usas recuerdas que no tenías un fondo para imprevistos.",
                        Skill::Resilience,
                        0,
                    );
                }), think("Bueno... se puede leer si lo inclino.")],
            ));
            vec![choice("La pantalla está trizada. ¿Qué haces?", opts)]
        })),
        act(Who::Sofia, Act::Release),
        doit(|s| {
            s.chars[0].held[1] = None;
            s.set_flag("week_beat_done");
            s.fin.add_goal("emergency", "Fondo para imprevistos", 20_000, None);
            s.toast("goal", "Nueva meta sugerida", "Fondo para imprevistos: $20.000", 0);
        }),
        think("Si hubiera tenido un fondo para imprevistos, esto habría sido mucho más fácil."),
    ]
}

fn repair_with_savings(s: &mut State) {
    let mut need = 24_990 - s.fin.wallet;
    if need > 0 {
        let from_sav = need.min(s.fin.savings);
        s.fin.from_savings(from_sav);
        need -= from_sav;
    }
    if need > 0 {
        let ids: Vec<&'static str> = s.fin.goals.iter().filter(|g| !g.done && g.saved > 0).map(|g| g.id).collect();
        for id in ids {
            if need <= 0 {
                break;
            }
            need -= s.fin.from_goal(id, need);
        }
    }
    s.fin.spend(24_990, "Reparación de pantalla");
    s.set_flag("repaired");
    fix_phone(s);
    s.fin.journal(
        "Tu ahorro te salvó",
        "Cuando se rompió tu teléfono, pagaste la reparación con tus ahorros. Sin deudas, sin intereses. Para eso sirve un fondo para imprevistos.",
        Skill::Resilience,
        2,
    );
    s.toast("check", "Teléfono reparado", "Pagaste con tus ahorros", -24_990);
}

fn fix_phone(s: &mut State) {
    s.flags.remove("phone_cracked");
    for c in s.chars.iter_mut().filter(|c| c.id == "sofia") {
        c.phone_cracked = 0.0;
    }
}

fn ask_mom_loan() -> Vec<Step> {
    vec![
        msg("mama", "Te presto la plata, hija. Pero me devuelves $5.000 cada semana, ¿trato?"),
        doit(|s| {
            s.fin.earn(24_990, "Préstamo de mamá");
            s.fin.spend(24_990, "Reparación de pantalla");
            s.fin.take_credit("Préstamo de mamá", 25_000, 5, 0.0, "mama");
            s.set_flag("repaired");
            s.set_flag("mom_loan");
            fix_phone(s);
            s.fin.journal(
                "Un préstamo familiar",
                "Mamá te prestó sin intereses. Fue una buena salida, pero tendrás $5.000 menos cada semana durante cinco semanas.",
                Skill::Resilience,
                0,
            );
        }),
        think("Trato. Cinco semanas pagándole a mamá..."),
    ]
}

fn week4() -> Vec<Step> {
    vec![
        act(Who::Sofia, Act::Stretch),
        msg("vale", "Hola Sofía, soy Vale del Café Aroma. Necesito ayuda los sábados en la tarde: $6.000 por turno + propinas. ¿Te interesa? Pasa por el café."),
        wait(1.0),
        msg("tomas", "Sofi!! Hoy en la tarde vamos al cine con los chicos. Entrada + cabritas ~$7.000. ¿Vienes?"),
        wait(0.8),
        Step::Expr(Who::Sofia, Expr::Thinking),
        think("Trabajar en el café... o ir al cine con todos. Las dos son en la tarde."),
        doit(|s| {
            s.set_flag("job_offer");
            s.set_flag("cinema_offer");
            s.set_flag("week_beat_done");
            s.objective = Some("Decide: trabajar en Café Aroma o ir al cine con Tomás (plaza)".into());
        }),
    ]
}

fn week5() -> Vec<Step> {
    vec![
        act(Who::Sofia, Act::Yawn),
        msg("tomas", "Sofi tengo una idea: ¿y si vendemos pulseras en la feria de la plaza? Como la que te regalé. Tú las diseñas, yo las vendo. ¡Socios!"),
        wait(0.9),
        Step::Expr(Who::Sofia, Expr::Excited),
        think("¿Un negocio? Tendríamos que comprar materiales, pagar el puesto, poner un precio..."),
        doit(|s| {
            s.set_flag("biz_offer");
            s.set_flag("week_beat_done");
            s.objective = Some("Habla con Tomás en la plaza sobre el negocio".into());
        }),
    ]
}

fn week6() -> Vec<Step> {
    vec![
        act(Who::Sofia, Act::Stretch),
        msg("banco", "Banco Futuro: ¡Hola Sofía! Ya puedes invertir desde tu app. Depósito a plazo (4 semanas, +3% fijo) o Fondo mutuo (variable). Invierte con responsabilidad."),
        wait(1.2),
        msg("desconocido", "¡¡FELICIDADES!! Ganaste un iPhone 16. Solo paga $4.990 de envío en el link: bit.ly/premio-seguro"),
        wait(0.6),
        Step::Expr(Who::Sofia, Expr::Surprised),
        choice(
            "Un mensaje dice que ganaste un iPhone. ¿Qué haces?",
            vec![
                opt(
                    "Pagar los $4.990 del envío",
                    vec![
                        doit(|s| {
                            if s.fin.spend(4_990, "Pago \"envío\" premio") {
                                s.fin.journal(
                                    "Caíste en una estafa",
                                    "Pagaste $4.990 por un premio que no existía. Si algo es demasiado bueno para ser verdad, probablemente no lo es.",
                                    Skill::Risk,
                                    -2,
                                );
                            }
                            s.set_flag("scammed");
                        }),
                        wait(1.5),
                        msg("desconocido", "..."),
                        think("No responden. No hay ningún iPhone. Me estafaron."),
                        Step::Expr(Who::Sofia, Expr::Sad),
                    ],
                ),
                opt(
                    "Ignorarlo y bloquear el número",
                    vec![
                        think("Nadie regala iPhones. Y nunca participé en ningún concurso."),
                        doit(|s| {
                            s.fin.journal(
                                "Detectaste una estafa",
                                "Ignoraste un mensaje de \"premio\" que pedía dinero. Los premios reales no te piden pagar para recibirlos.",
                                Skill::Risk,
                                2,
                            );
                        }),
                    ],
                ),
                opt(
                    "Preguntarle a mamá",
                    vec![
                        msg("mama", "¡Es una estafa, Sofi! Nunca pagues por un premio. Bloquea ese número."),
                        doit(|s| {
                            s.fin.journal(
                                "Pediste consejo",
                                "Antes de pagar algo sospechoso, preguntaste. Pedir ayuda también es una habilidad financiera.",
                                Skill::Risk,
                                1,
                            );
                        }),
                    ],
                ),
            ],
        ),
        doit(|s| {
            s.set_flag("invest_open");
            s.set_flag("week_beat_done");
            s.objective = Some("Opcional: habla con Tomás sobre \"MoonCoin\" o revisa la app de inversiones".into());
        }),
    ]
}

fn week7() -> Vec<Step> {
    vec![
        act(Who::Sofia, Act::Stretch),
        msg("tomas", "¡¡Anunciaron el concierto de Nube Rosa!! Entradas a $35.000. Todos van. Compra la tuya YA que se agotan."),
        wait(1.0),
        Step::Expr(Who::Sofia, Expr::Excited),
        think("¡Nube Rosa! Me encanta... pero $35.000 es más de la mitad de lo que necesito para el viaje."),
        Step::Dyn(Box::new(|s| {
            let avail = s.fin.wallet;
            let mut opts = vec![];
            if avail >= 35_000 {
                opts.push(opt_d("Comprar la entrada ($35.000)", "Con tu billetera", vec![doit(|s| concert(s, 0))]));
            } else {
                opts.push(opt_locked("Comprar la entrada ($35.000)", &format!("Tienes {} en la billetera", money(avail))));
            }
            opts.push(opt_d("Comprarla en 4 cuotas", "4 cuotas de $9.990 = $39.960", vec![doit(|s| concert(s, 1))]));
            opts.push(opt_d(
                "No ir y ver la transmisión en casa con amigas",
                "Picoteo compartido: $3.000",
                vec![doit(|s| concert(s, 2))],
            ));
            opts.push(opt("No ir", vec![doit(|s| concert(s, 3))]));
            vec![choice("¿Vas al concierto?", opts)]
        })),
        doit(|s| {
            s.set_flag("week_beat_done");
        }),
    ]
}

fn concert(s: &mut State, choice: u32) {
    match choice {
        0 => {
            s.fin.spend(35_000, "Entrada concierto Nube Rosa");
            s.fin.mood += 0.3;
            s.set_flag("concert");
            let trip = s.fin.goals.iter().find(|g| g.id == "trip").map(|g| g.target - g.saved).unwrap_or(0);
            s.fin.journal(
                "El concierto",
                &format!("Pagaste $35.000 al contado por el concierto. Lo disfrutaste, y no quedaste con deudas. Al viaje le faltaban {}.", money(trip)),
                Skill::Planning,
                0,
            );
        }
        1 => {
            s.fin.take_credit("Entrada concierto", 35_000, 4, 0.0555, "tecno");
            s.fin.mood += 0.25;
            s.set_flag("concert");
            s.fin.journal(
                "Concierto en cuotas",
                "Compraste la entrada en 4 cuotas. Pagarás $39.960 por algo que dura una noche, y las cuotas seguirán después del concierto.",
                Skill::Credit,
                -1,
            );
        }
        2 => {
            s.fin.spend(3_000, "Picoteo transmisión");
            s.fin.mood += 0.12;
            s.fin.journal(
                "Una alternativa más barata",
                "En vez de $35.000, compartiste la transmisión con amigas por $3.000. Buscar alternativas también es comparar.",
                Skill::Comparing,
                2,
            );
        }
        _ => {
            s.fin.mood -= 0.1;
            s.fin.journal(
                "Decir que no",
                "Decidiste no ir al concierto para cuidar tu meta. Dolió un poco ver las fotos de todos, pero seguiste tu plan.",
                Skill::Planning,
                1,
            );
        }
    }
}

fn week8(s: &State) -> Vec<Step> {
    let _ = s;
    vec![
        act(Who::Sofia, Act::Stretch),
        msg("colegio", "Recordatorio: este lunes se paga el viaje de estudios. Tu parte: $60.000."),
        wait(0.8),
        Step::Dyn(Box::new(|s| {
            let mut v = vec![];
            if s.fin.investments.iter().any(|i| i.kind == InvestKind::Crypto) {
                v.push(msg("tomas", "Sofi... MoonCoin se desplomó. Perdí casi todo. ¿Tú alcanzaste a vender?"));
            }
            let trip_saved = s.fin.goals.iter().find(|g| g.id == "trip").map(|g| g.saved).unwrap_or(0);
            let total = s.fin.wallet + s.fin.savings + s.fin.goals_saved();
            v.push(think(&format!(
                "Tengo {} apartados para el viaje, y {} en total entre billetera, ahorro y metas.",
                money(trip_saved),
                money(total)
            )));
            let mut opts = vec![];
            if total >= 60_000 {
                opts.push(opt_d("Pagar el viaje ($60.000)", "Usando tu meta, ahorro y billetera", vec![doit(|s| pay_trip(s))]));
            } else {
                opts.push(opt_locked("Pagar el viaje ($60.000)", &format!("Te faltan {}", money(60_000 - total))));
                opts.push(opt_d("Pedir un crédito de consumo", "Préstamo de $60.000 en 8 cuotas de $8.990", vec![doit(|s| {
                    s.fin.take_credit("Crédito viaje", 60_000, 8, 0.03, "banco");
                    s.fin.earn(60_000, "Crédito de consumo");
                    pay_trip(s);
                    s.set_flag("trip_on_credit");
                    s.fin.journal(
                        "El viaje a crédito",
                        "No alcanzaste a juntar el dinero y pediste un crédito. Irás al viaje, pero pagarás $71.920 durante ocho semanas.",
                        Skill::Credit,
                        -1,
                    );
                })]));
                opts.push(opt_d("No ir al viaje", "", vec![doit(|s| {
                    s.set_flag("no_trip");
                    s.fin.journal(
                        "Me quedé sin viaje",
                        "No juntaste los $60.000 a tiempo. La próxima vez, empezar antes y apartar cada semana puede marcar la diferencia.",
                        Skill::Planning,
                        -2,
                    );
                })]));
            }
            v.push(choice("El viaje de estudios", opts));
            v
        })),
        doit(|s| s.set_flag("week_beat_done")),
        Step::Dyn(Box::new(|_| ending())),
    ]
}

fn pay_trip(s: &mut State) {
    let mut need = 60_000i64;
    if let Some(g) = s.fin.goals.iter_mut().find(|g| g.id == "trip" && !g.done) {
        let use_g = g.saved.min(need);
        g.saved -= use_g;
        need -= use_g;
        g.done = true;
        s.fin.log("Pago viaje (meta)", -use_g, super::finance::Account::Goal);
    }
    if need > 0 {
        let a = need.min(s.fin.wallet);
        s.fin.wallet -= a;
        need -= a;
        s.fin.log("Pago viaje", -a, super::finance::Account::Wallet);
    }
    if need > 0 {
        let a = need.min(s.fin.savings);
        s.fin.savings -= a;
        need -= a;
        s.fin.log("Pago viaje (ahorro)", -a, super::finance::Account::Savings);
    }
    if need > 0 {
        let ids: Vec<&'static str> = s.fin.goals.iter().filter(|g| !g.done && g.saved > 0).map(|g| g.id).collect();
        for id in ids {
            if need <= 0 {
                break;
            }
            if let Some(g) = s.fin.goal_mut(id) {
                let a = g.saved.min(need);
                g.saved -= a;
                need -= a;
            }
        }
    }
    s.set_flag("trip_paid");
    if !s.flag("trip_on_credit") {
        s.fin.journal(
            "¡Me voy de viaje!",
            "Pagaste tu parte del viaje de estudios con lo que fuiste juntando semana a semana. Nadie te lo regaló: lo planificaste tú.",
            Skill::Planning,
            2,
        );
    }
    let p = s.chars[0].anim.pos + Vec3::Y * 1.6;
    s.burst(1, p);
}

fn ending() -> Vec<Step> {
    vec![
        Step::Fade(true, 1.0),
        doit(|s| {
            s.goto(Loc::Plaza, "fountain");
            s.set_phase(Phase::Evening);
            s.hour = 19.3;
            s.place(Who::Mama, Loc::Plaza, "fountain");
            let p = s.chars[0].anim.pos;
            if let Some(m) = s.ch(Who::Mama) {
                m.anim.teleport(p + Vec3::new(1.0, 0.0, 0.1), -1.4);
            }
            if let Some(t) = s.ch(Who::Tomas) {
                t.anim.teleport(p + Vec3::new(-1.0, 0.0, 0.2), 1.4);
            }
            if let Some(ti) = s.char_idx(Who::Tomas) {
                s.char_loc[ti] = Loc::Plaza;
            }
        }),
        Step::Letterbox(true),
        Step::Fade(false, 1.5),
        shot(ShotKind::Wide),
        Step::Title("Ocho semanas después".into(), "".into()),
        shot(ShotKind::Two(Who::Sofia, Who::Mama)),
        Step::Dyn(Box::new(|s| {
            let mut v = vec![];
            if s.flag("trip_paid") {
                v.push(say_e(Who::Mama, Expr::Happy, "Así que te vas de viaje. Estoy orgullosa de ti, Sofi."));
                v.push(say_e(Who::Sofia, Expr::Happy, "Fue difícil. Hubo semanas en que quería gastarlo todo."));
            } else {
                v.push(say_e(Who::Mama, Expr::Worried, "Siento que no puedas ir al viaje, hija."));
                v.push(say_e(Who::Sofia, Expr::Sad, "Yo también. Pero creo que entendí qué hice mal."));
            }
            v.push(say(Who::Mama, "¿Y qué aprendiste en todo este tiempo?"));
            v.push(shot(ShotKind::Close(Who::Sofia)));
            v.push(say_e(Who::Sofia, Expr::Thinking, "Que la plata no es solo para gastar o guardar. Cada decisión tiene consecuencias... a veces semanas después."));
            if s.fin.debts.is_empty() {
                v.push(say(Who::Sofia, "Y que no deber nada se siente muy bien."));
            } else {
                v.push(say(Who::Sofia, "Y que las cuotas siguen ahí cuando la emoción de comprar ya pasó."));
            }
            v.push(shot(ShotKind::Two(Who::Sofia, Who::Tomas)));
            v.push(say_e(Who::Tomas, Expr::Smug, "Yo aprendí que si algo \"se va a multiplicar por diez\", mejor desconfiar."));
            v.push(act(Who::Sofia, Act::Celebrate));
            v
        })),
        Step::Modal(Modal::Reflection),
        Step::Title("Fin del capítulo 8".into(), "La vida continúa: modo libre".into()),
        doit(|s| {
            s.set_flag("ending_seen");
            s.objective = Some("Modo libre: sigue viviendo, ahorrando y decidiendo.".into());
            s.place(Who::Mama, Loc::Home, "mom");
            s.place(Who::Tomas, Loc::Plaza, "tomas");
        }),
    ]
}

fn free_week(s: &State) -> Vec<Step> {
    let mut v = vec![act(Who::Sofia, Act::Stretch)];
    let mut rng = crate::math::Rng::new(s.week as u64 * 97 + 3);
    match rng.int(5) {
        0 => {
            v.push(msg("abuela", "Hola mi niña, te mandé $5.000 para la semana. ¡Cuídalos!"));
            v.push(doit(|s| s.fin.earn(5_000, "Regalo de la abuela")));
        }
        1 => {
            v.push(msg("tecno", "¡CYBER! 30% de descuento en toda la tienda solo este sábado."));
            v.push(doit(|s| s.set_flag("flash_sale")));
        }
        2 => {
            v.push(think("La mochila del colegio se rompió. Una nueva cuesta $8.000."));
            v.push(doit(|s| {
                if s.fin.spend(8_000, "Mochila nueva") {
                    s.toast("bag", "Gasto imprevisto", "Mochila nueva", -8_000);
                } else {
                    s.toast("warning", "Sin dinero", "Tendrás que usar la mochila rota", 0);
                }
            }));
        }
        3 => {
            v.push(msg("vale", "¡Hoy hay mucha gente en el café! Si vienes a trabajar, las propinas van a estar buenas."));
            v.push(doit(|s| s.set_flag("busy_cafe")));
        }
        _ => {
            v.push(think("Un sábado tranquilo. ¿Qué haré hoy?"));
        }
    }
    v.push(doit(|s| s.set_flag("week_beat_done")));
    v
}

// ------------------------------------------------------------------ conversations

pub fn talk(s: &mut State, who: Who) -> Option<Vec<Step>> {
    let mut v = vec![
        Step::Letterbox(true),
        face(Who::Sofia, Target::Char(who)),
        face(who, Target::Char(Who::Sofia)),
        Step::Look(Who::Sofia, Some(Target::Char(who))),
        Step::Look(who, Some(Target::Char(Who::Sofia))),
        shot(ShotKind::Two(Who::Sofia, who)),
    ];
    let body = match who {
        Who::Mama => talk_mom(s),
        Who::Tomas => talk_tomas(s),
        Who::Vale => talk_vale(s),
        Who::Julio => talk_julio(s),
        _ => vec![],
    };
    v.extend(body);
    v.push(Step::Look(Who::Sofia, None));
    v.push(Step::Look(who, None));
    Some(v)
}

fn talk_mom(s: &mut State) -> Vec<Step> {
    if s.week == 1 && !s.flag("w1_mom") {
        return mom_birthday();
    }
    if s.flag("mom_loan") && s.fin.debts.iter().any(|d| d.lender == "mama") {
        return vec![
            say(Who::Mama, "¿Cómo vas con lo que me debes? No te preocupes, sé que vas a cumplir."),
            say(Who::Sofia, "Cada semana te devuelvo $5.000, mamá."),
        ];
    }
    let lines = [
        "Recuerda: primero lo necesario, después los gustos.",
        "¿Cómo va tu meta? A veces ayuda ponerle una foto en el tablero.",
        "Antes de comprar algo, espera un día. Si al día siguiente todavía lo quieres, es porque de verdad lo quieres.",
        "Las cuotas son cómodas, pero siempre revisa cuánto pagas en total.",
    ];
    let i = (s.week as usize + s.time as usize) % lines.len();
    vec![say(Who::Mama, lines[i]), act(Who::Mama, Act::Nod)]
}

fn mom_birthday() -> Vec<Step> {
    vec![
        act_nb(Who::Mama, Act::Wave),
        say_e(Who::Mama, Expr::Joy, "¡Feliz cumpleaños, hija! Quince años... ¿cuándo pasó tan rápido?"),
        say_e(Who::Sofia, Expr::Happy, "¡Gracias, mamá! ¿Sabías que la abuela me transfirió treinta mil?"),
        shot(ShotKind::Over(Who::Sofia, Who::Mama)),
        say(Who::Mama, "Me contó. Está feliz de que ya tengas tu propia cuenta."),
        say(Who::Mama, "Y hay otra novedad: desde esta semana vas a recibir una mesada de $10.000."),
        say(Who::Mama, "Con eso pagas tus colaciones y la micro. Lo que te sobre, lo administras tú."),
        shot(ShotKind::Close(Who::Sofia)),
        think("Diez mil a la semana... y treinta mil hoy."),
        shot(ShotKind::Over(Who::Sofia, Who::Mama)),
        say_e(Who::Mama, Expr::Smug, "Entonces, cumpleañera... ¿qué vas a hacer con la plata de tu abuela?"),
        shot(ShotKind::PushIn(Who::Sofia)),
        Step::Expr(Who::Sofia, Expr::Thinking),
        choice(
            "¿Qué harás con los $30.000?",
            vec![
                opt_d(
                    "Gastarlo todo en algo que quiero",
                    "Un gusto ahora",
                    vec![
                        say_e(Who::Sofia, Expr::Excited, "¡Me voy a comprar algo increíble! Hay ofertas en TecnoMundo."),
                        say(Who::Mama, "Es tu plata y tu decisión. Solo pregúntate si lo que compras vale lo que cuesta."),
                        doit(|s| s.set_flag("plan_spend")),
                    ],
                ),
                opt_d(
                    "Gastar una parte y ahorrar el resto",
                    "Tú eliges cuánto",
                    vec![
                        say(Who::Sofia, "Voy a guardar una parte... y darme un gusto con el resto."),
                        Step::Modal(Modal::Amount(AmountState {
                            title: "¿Cuánto quieres ahorrar?".into(),
                            body: "Lo que ahorres irá a tu cuenta de ahorro (0,5% de interés semanal). El resto queda en tu billetera.".into(),
                            min: 1_000,
                            max: 29_000,
                            step: 1_000,
                            value: 15_000,
                            confirm: "Ahorrar".into(),
                            on_ok: Some(Box::new(|s: &mut State, v| {
                                s.fin.to_savings(v);
                                s.set_flag("plan_split");
                                s.fin.journal(
                                    "El regalo de la abuela",
                                    &format!("Ahorraste {} y dejaste {} para darte un gusto.", money(v), money(30_000 - v)),
                                    Skill::Planning,
                                    1,
                                );
                            })),
                            cancel: false,
                        })),
                        say(Who::Mama, "Me parece equilibrado. Darse gustos también es parte de la vida."),
                    ],
                ),
                opt_d(
                    "Ahorrarlo todo",
                    "A la cuenta de ahorro",
                    vec![
                        say(Who::Sofia, "Lo voy a ahorrar todo."),
                        doit(|s| {
                            s.fin.to_savings(30_000);
                            s.set_flag("plan_save");
                            s.fin.journal(
                                "El regalo de la abuela",
                                "Ahorraste los $30.000 completos. Todavía sin un objetivo claro, pero con una base.",
                                Skill::Planning,
                                1,
                            );
                        }),
                        say_e(Who::Mama, Expr::Surprised, "¡Wow! ¿Y para qué lo vas a guardar?"),
                        say_e(Who::Sofia, Expr::Thinking, "Mmm... no sé todavía. Solo quiero tenerlo."),
                        say(Who::Mama, "Ahorrar es bueno. Con una meta, cuesta menos no gastarlo."),
                    ],
                ),
                opt_d(
                    "Usarlo para una meta",
                    "Apartar para algo importante",
                    vec![
                        say(Who::Sofia, "Quiero juntar para algo importante."),
                        Step::Modal(Modal::GoalPicker(GoalPick {
                            on_pick: Some(Box::new(|s: &mut State, id| {
                                s.set_flag("plan_goal");
                                let name = s.fin.goals.iter().find(|g| g.id == id).map(|g| g.name.clone()).unwrap_or_default();
                                s.fin.journal(
                                    "El regalo de la abuela",
                                    &format!("Usaste el regalo para empezar tu meta: {name}. Cada peso tuvo un propósito desde el primer día."),
                                    Skill::Planning,
                                    2,
                                );
                            })),
                            fund_with: 30_000,
                        })),
                        say_e(Who::Mama, Expr::Happy, "¡Me encanta! Con una meta clara, cada peso tiene un propósito."),
                    ],
                ),
                opt_locked("Invertirlo", "Todavía no sabes cómo funcionan las inversiones"),
            ],
        ),
        shot(ShotKind::Two(Who::Sofia, Who::Mama)),
        say(Who::Mama, "Bueno, ¡disfruta tu día! Hoy no hay tareas."),
        act_nb(Who::Sofia, Act::Nod),
        doit(|s| {
            s.set_flag("w1_mom");
            s.hint("Abre tu teléfono (botón inferior derecho o TAB) para ver tu banco y tus metas.");
        }),
        wait(0.6),
        msg("tomas", "¡¡Feliz cumple Sofi!! Ven a la plaza en la tarde, te tengo un regalo. Y hay ofertas en TecnoMundo..."),
        objective("Ve a la plaza (sal por la puerta principal)"),
    ]
}

fn talk_tomas(s: &mut State) -> Vec<Step> {
    if s.week == 1 && !s.flag("w1_tomas") {
        return vec![
            say_e(Who::Tomas, Expr::Excited, "¡Cumpleañeraaa! Toma, te hice esto."),
            act(Who::Tomas, Act::Give),
            Step::Held(Who::Sofia, 0, Some(HeldProp::Bracelet)),
            act(Who::Sofia, Act::Receive),
            say_e(Who::Sofia, Expr::Joy, "¡Una pulsera! ¿La hiciste tú?"),
            Step::Held(Who::Sofia, 0, None),
            say_e(Who::Tomas, Expr::Smug, "Con hilos que tenía en la casa. Oye, mira mis zapatillas nuevas."),
            shot(ShotKind::Close(Who::Tomas)),
            say(Who::Tomas, "Las compré en seis cuotas. Casi ni se siente: pago poquito cada semana."),
            shot(ShotKind::Close(Who::Sofia)),
            think("Poquito cada semana... durante seis semanas."),
            shot(ShotKind::Two(Who::Sofia, Who::Tomas)),
            say_e(Who::Tomas, Expr::Excited, "¿Y tú qué te vas a comprar? En TecnoMundo están los audífonos Pro en oferta, ¡a $39.990!"),
            Step::Dyn(Box::new(|s| {
                if s.flag("plan_spend") || s.flag("plan_split") {
                    vec![say_e(Who::Sofia, Expr::Excited, "¡Justo los quería ver!")]
                } else {
                    vec![
                        say(Who::Sofia, "Mmm, estoy juntando plata."),
                        say_e(Who::Tomas, Expr::Annoyed, "¡Qué aburrida! Bueno, mirar es gratis."),
                    ]
                }
            })),
            doit(|s| {
                s.set_flag("w1_tomas");
                s.set_flag("met_all");
                s.objective = Some("Opcional: visita TecnoMundo. Luego vuelve a casa y duerme.".into());
            }),
        ];
    }
    if s.flag("cinema_offer") && !s.flag("cinema_decided") {
        return vec![
            say_e(Who::Tomas, Expr::Excited, "¿Y? ¿Vienes al cine? Salimos en un rato."),
            choice(
                "¿Qué haces esta tarde?",
                vec![
                    opt_d(
                        "Ir al cine con Tomás",
                        "Entrada + cabritas: $7.000",
                        vec![
                            doit(|s| {
                                s.set_flag("cinema_decided");
                                if s.fin.spend(7_000, "Cine con amigos") {
                                    s.fin.mood += 0.2;
                                    s.fin.journal(
                                        "Tarde de cine",
                                        "Elegiste compartir con tus amigos en vez de trabajar. Gastaste $7.000 y ganaste un buen recuerdo. El tiempo también tiene valor.",
                                        Skill::Planning,
                                        0,
                                    );
                                }
                            }),
                            say_e(Who::Tomas, Expr::Joy, "¡Vamos!"),
                            Step::Fade(true, 1.0),
                            doit(|s| s.set_phase(Phase::Evening)),
                            Step::Fade(false, 1.0),
                            think("La película estuvo buenísima. Y las cabritas, carísimas."),
                        ],
                    ),
                    opt_d(
                        "Trabajar en el café",
                        "Ganar $6.000 + propinas",
                        vec![
                            say(Who::Sofia, "Esta vez no. Voy a probar trabajando en el café."),
                            say_e(Who::Tomas, Expr::Sad, "Buuu. Bueno, ¡suerte en tu primer día!"),
                            doit(|s| {
                                s.set_flag("cinema_decided");
                                s.objective = Some("Ve al Café Aroma y habla con Vale".into());
                            }),
                        ],
                    ),
                ],
            ),
        ];
    }
    if s.flag("biz_offer") && !s.biz.active {
        return vec![
            say_e(Who::Tomas, Expr::Excited, "¿Viste mi mensaje? ¡Pulseras en la feria! Yo tengo el contacto del puesto."),
            say(Who::Tomas, "El puesto cuesta $2.000 por sábado. Los materiales para 10 pulseras, $8.000; para 30, $19.500."),
            say(Who::Sofia, "Si compramos más, cada pulsera sale más barata... pero tenemos que venderlas."),
            choice(
                "¿Emprenden juntos?",
                vec![
                    opt_d(
                        "Ser socios 50/50",
                        "Compartir costos y ganancias con Tomás",
                        vec![
                            doit(|s| {
                                s.biz.active = true;
                                s.biz.partner = true;
                                s.set_flag("biz_started");
                                s.objective = Some("Ve al puesto de la feria para preparar tu negocio".into());
                            }),
                            say_e(Who::Tomas, Expr::Joy, "¡Socios! Yo vendo, tú diseñas. Mitad y mitad de todo."),
                        ],
                    ),
                    opt_d(
                        "Hacerlo sola",
                        "Toda la ganancia (y todo el riesgo) es tuya",
                        vec![
                            doit(|s| {
                                s.biz.active = true;
                                s.biz.partner = false;
                                s.set_flag("biz_started");
                                s.objective = Some("Ve al puesto de la feria para preparar tu negocio".into());
                            }),
                            say_e(Who::Tomas, Expr::Sad, "Ah... bueno. Igual te ayudo a armar el puesto."),
                        ],
                    ),
                    opt("Ahora no", vec![say(Who::Tomas, "Piénsalo. La feria es todos los sábados.")]),
                ],
            ),
        ];
    }
    if s.flag("invest_open") && !s.flag("moon_talk") {
        return vec![
            say_e(Who::Tomas, Expr::Excited, "Sofi, tengo el dato del siglo: MoonCoin. Subió 40% este mes."),
            say(Who::Tomas, "Yo metí todos mis ahorros. Dicen que se va a multiplicar por diez."),
            say_e(Who::Sofia, Expr::Thinking, "¿Quién dice eso?"),
            say_e(Who::Tomas, Expr::Smug, "Un youtuber que sabe mucho. Tiene un Lamborghini."),
            think("Si fuera tan seguro, ¿no lo sabría todo el mundo?"),
            doit(|s| {
                s.set_flag("moon_talk");
                s.hint("En tu teléfono, la app Inversiones muestra cada opción con su nivel de riesgo.");
            }),
        ];
    }
    let lines = [
        "¿Viste lo caro que está todo? Mi mesada no me dura ni tres días.",
        "Todavía me quedan cuotas de las zapatillas. Ya ni me gustan tanto.",
        "Si algún día quieres emprender algo, cuenta conmigo.",
        "Oye, ¿me prestas mil? Es broma. O no. Es broma.",
    ];
    let i = (s.week as usize * 3 + s.time as usize) % lines.len();
    vec![say(Who::Tomas, lines[i])]
}

fn talk_vale(s: &mut State) -> Vec<Step> {
    if s.flag("job_offer") && !s.flag("job") {
        return vec![
            say_e(Who::Vale, Expr::Happy, "¡Sofía! ¿Viste mi mensaje? Necesito ayuda los sábados en la tarde."),
            say(Who::Vale, "Son $6.000 por turno, más propinas. Tienes que ser rápida y amable."),
            choice(
                "¿Aceptas el trabajo?",
                vec![
                    opt(
                        "Acepto",
                        vec![
                            doit(|s| {
                                s.set_flag("job");
                                s.set_flag("cinema_decided");
                                s.fin.journal(
                                    "Primer trabajo",
                                    "Aceptaste trabajar los sábados en Café Aroma. Tu tiempo se convirtió en ingresos.",
                                    Skill::Enterprise,
                                    1,
                                );
                            }),
                            say_e(Who::Vale, Expr::Joy, "¡Genial! Ponte el delantal: tu primer turno es ahora."),
                            Step::Dyn(Box::new(|_| work_shift())),
                        ],
                    ),
                    opt("Ahora no", vec![say(Who::Vale, "La oferta sigue en pie. Avísame.")]),
                ],
            ),
        ];
    }
    if s.flag("job") && !s.flag("worked_today") {
        return vec![
            say(Who::Vale, "¿Lista para tu turno?"),
            choice(
                "Turno en Café Aroma",
                vec![opt("Trabajar ahora ($6.000 + propinas)", work_shift()), opt("Hoy no", vec![])],
            ),
        ];
    }
    vec![
        say(Who::Vale, "¿Te sirvo algo? El latte de vainilla es el favorito."),
        doit(|s| {
            s.modal = Some(Modal::Shop(ShopState {
                shop: Shop::Cafe,
                selected: None,
                message: None,
                compare: false,
            }));
        }),
    ]
}

fn work_shift() -> Vec<Step> {
    vec![
        Step::Fade(true, 0.6),
        doit(|s| {
            s.set_flag("worked_today");
            let busy = s.flag("busy_cafe");
            s.modal = Some(Modal::Minigame(minigame::CafeGame::new(busy, s.week as u64)));
        }),
        Step::Fade(false, 0.6),
        doit(|s| s.dir.waiting = Waiting::Modal),
        doit(|s| s.set_phase(Phase::Evening)),
    ]
}

fn talk_julio(s: &mut State) -> Vec<Step> {
    let _ = s;
    vec![
        say(Who::Julio, "¡Buenas! En TecnoMundo tenemos de todo. Y si no te alcanza, ¡tenemos cuotas!"),
        think("Siempre ofrece cuotas... ¿por qué será?"),
    ]
}

// ------------------------------------------------------------------ places

fn store(s: &mut State) -> Vec<Step> {
    let mut v = vec![];
    if !s.flag("store_visited") {
        v.push(Step::Letterbox(true));
        if s.present(Who::Julio) {
            v.push(face(Who::Julio, Target::Char(Who::Sofia)));
            v.push(shot(ShotKind::Two(Who::Sofia, Who::Julio)));
            v.push(say_e(Who::Julio, Expr::Happy, "¡Bienvenida a TecnoMundo! Hoy tenemos ofertas imperdibles."));
            v.push(say(Who::Julio, "Y con la Tarjeta TecnoMundo te lo llevas hoy y pagas en cuotas."));
        }
        v.push(doit(|s| s.set_flag("store_visited")));
    }
    v.push(doit(|s| {
        s.modal = Some(Modal::Shop(ShopState {
            shop: Shop::TecnoMundo,
            selected: None,
            message: None,
            compare: false,
        }));
    }));
    v.push(doit(|s| s.dir.waiting = Waiting::Modal));
    v
}

fn cafe(s: &mut State) -> Vec<Step> {
    if s.present(Who::Vale) {
        return talk(s, Who::Vale).unwrap_or_default();
    }
    vec![doit(|s| {
        s.modal = Some(Modal::Shop(ShopState {
            shop: Shop::Cafe,
            selected: None,
            message: None,
            compare: false,
        }));
    })]
}

fn bank(s: &mut State) -> Vec<Step> {
    let _ = s;
    vec![
        think("Banco Futuro. Desde el cajero puedo ver mis cuentas."),
        Step::Modal(Modal::Phone(PhoneApp::Bank)),
    ]
}

fn stall(s: &mut State) -> Vec<Step> {
    if !s.biz.active {
        if s.flag("biz_offer") {
            return vec![think("Aquí podríamos poner el puesto de pulseras. Primero hablo con Tomás.")];
        }
        return vec![
            think("Un puesto de la feria. Venden cosas hechas a mano."),
            doit(|s| {
                s.modal = Some(Modal::Shop(ShopState {
                    shop: Shop::Feria,
                    selected: None,
                    message: None,
                    compare: false,
                }));
            }),
        ];
    }
    if s.flag("sold_today") {
        return vec![think("Ya vendimos hoy. El próximo sábado volvemos.")];
    }
    vec![
        doit(|s| {
            s.modal = Some(Modal::Business(business::BizUi::new(&s.biz)));
            s.loc_mut(Loc::Plaza).set_prop_visible("stall_goods", true);
        }),
        doit(|s| s.dir.waiting = Waiting::Modal),
    ]
}

// ------------------------------------------------------------------ economy hooks

pub fn buy(s: &mut State, idx: usize, credit: Option<(u32, f32)>) {
    let shop = match &s.modal {
        Some(Modal::Shop(st)) => st.shop,
        _ => return,
    };
    let list: Vec<items::Item> = shop_items(s, shop);
    let Some(it) = list.get(idx).cloned() else {
        return;
    };
    if s.fin.inventory.contains(&it.id) && !it.consumable {
        set_shop_msg(s, "Ya lo tienes.", false);
        return;
    }
    let mut price = it.price;
    if it.id == "speaker" && s.flag("flash_sale") {
        price = 12_990;
    }
    if s.flag("flash_sale") && s.week > LAST_WEEK && it.shop == Shop::TecnoMundo {
        price = price * 7 / 10;
    }
    // money reserved for a goal with the same id
    let goal_money = s.fin.goals.iter().find(|g| g.id == it.id && !g.done).map(|g| g.saved).unwrap_or(0);
    match credit {
        None => {
            if s.fin.wallet + goal_money < price {
                set_shop_msg(
                    s,
                    &format!(
                        "No te alcanza: tienes {} en la billetera. Puedes mover dinero desde tu ahorro en la app del banco.",
                        money(s.fin.wallet)
                    ),
                    false,
                );
                return;
            }
            if goal_money > 0 {
                let used = goal_money.min(price);
                if let Some(g) = s.fin.goal_mut(it.id) {
                    g.saved -= used;
                    if g.saved <= 0 {
                        g.done = true;
                    }
                }
                s.fin.wallet += used;
            }
            s.fin.spend(price, &format!("Compra: {}", it.name));
            if goal_money >= price {
                s.fin.journal(
                    &format!("Meta cumplida: {}", it.name),
                    "Compraste algo que planificaste y ahorraste para ello. Sin deudas y sin culpa.",
                    Skill::Planning,
                    2,
                );
            }
        }
        Some((n, rate)) => {
            let inst = s.fin.take_credit(it.name, price, n, rate, "tecno");
            let total = inst * n as i64;
            if rate > 0.0 {
                s.fin.journal(
                    &format!("{} en {} cuotas", it.name, n),
                    &format!(
                        "Compraste a crédito: {} cuotas de {} = {} en total. Pagaste {} extra por no tener el dinero hoy.",
                        n,
                        money(inst),
                        money(total),
                        money(total - price)
                    ),
                    Skill::Credit,
                    -1,
                );
            } else {
                s.fin.journal(
                    &format!("{} en cuotas sin interés", it.name),
                    "Compraste en cuotas sin interés. No pagaste extra, pero comprometiste tu mesada de las próximas semanas.",
                    Skill::Credit,
                    0,
                );
            }
        }
    }
    if !it.consumable {
        s.fin.inventory.push(it.id);
    }
    s.fin.mood += it.joy;
    if s.flag("compared_prices") && !it.consumable && !s.flag("compare_journal") {
        s.set_flag("compare_journal");
        s.fin.journal(
            "Comparaste antes de comprar",
            "Antes de decidir, comparaste precios, cuotas y el costo total. Esa es una de las habilidades más valiosas.",
            Skill::Comparing,
            1,
        );
    }
    if let Some(p) = it.prop {
        s.loc_mut(Loc::Bedroom).set_prop_visible(p, true);
    }
    if it.consumable {
        if let Some(c) = s.chars.first_mut() {
            c.held[1] = Some(HeldProp::Cup);
        }
        if it.id == "latte" {
            s.set_flag("latte_count");
        }
    }
    let p = s.chars[0].anim.pos + Vec3::Y * 1.2;
    s.burst(0, p);
    set_shop_msg(s, &format!("¡Compraste {}!", it.name), true);
    s.toast("bag", "Compra", it.name, -price);
    check_goals(s);
}

fn set_shop_msg(s: &mut State, m: &str, ok: bool) {
    if let Some(Modal::Shop(st)) = &mut s.modal {
        st.message = Some((m.to_string(), ok));
    }
}

pub fn shop_items(s: &State, shop: Shop) -> Vec<items::Item> {
    let _ = s;
    items::catalog().into_iter().filter(|i| i.shop == shop).collect()
}

pub fn goal_options() -> Vec<(&'static str, &'static str, i64, Option<u32>)> {
    vec![
        ("trip", "Viaje de estudios (tu parte)", 60_000, Some(LAST_WEEK)),
        ("headphones_pro", "Audífonos inalámbricos Pro", 39_990, None),
        ("bike", "Bicicleta urbana", 149_990, None),
        ("guitar", "Guitarra acústica", 89_990, None),
        ("emergency", "Fondo para imprevistos", 20_000, None),
    ]
}

pub fn create_goal(s: &mut State, id: &'static str) {
    if let Some((id, name, target, deadline)) = goal_options().into_iter().find(|g| g.0 == id) {
        s.fin.add_goal(id, name, target, deadline);
        s.toast("target", "Nueva meta", name, 0);
        refresh_room(s);
    }
}

pub fn check_goals(s: &mut State) {
    let reached: Vec<(String, &'static str)> = s
        .fin
        .goals
        .iter()
        .filter(|g| !g.done && g.saved >= g.target)
        .map(|g| (g.name.clone(), g.id))
        .collect();
    for (name, id) in reached {
        let key = format!("goal_reached_{id}");
        if s.flag(&key) {
            continue;
        }
        s.set_flag(&key);
        s.toast("star", "¡Meta alcanzada!", &name, 0);
        let p = s.chars[0].anim.pos + Vec3::Y * 1.7;
        s.burst(1, p);
        s.flash = 0.15;
        if id == "emergency" {
            s.fin.journal(
                "Fondo para imprevistos listo",
                "Juntaste $20.000 para emergencias. La próxima sorpresa no te tomará desprevenida.",
                Skill::Resilience,
                2,
            );
        }
        if s.dir.queue.is_empty() && s.modal.is_none() {
            s.dir.run(vec![act(Who::Sofia, Act::Celebrate)]);
        }
    }
}

pub fn open_invest(s: &mut State, kind: InvestKind) {
    if !s.flag("invest_open") {
        s.toast("lock", "Inversiones", "Se desbloquean más adelante en la historia", 0);
        return;
    }
    let (title, body) = match kind {
        InvestKind::Deposit => (
            "Depósito a plazo (4 semanas)",
            "Ganas 3% fijo al final. Riesgo: muy bajo. No puedes sacar el dinero antes de 4 semanas.",
        ),
        InvestKind::Fund => (
            "Fondo mutuo moderado",
            "Rentabilidad variable: algunas semanas sube, otras baja. En promedio crece de a poco. Puedes rescatarlo cuando quieras.",
        ),
        InvestKind::Crypto => (
            "MoonCoin",
            "Criptomoneda muy volátil. Puede subir mucho... o caer casi a cero. Nadie te garantiza nada.",
        ),
    };
    let max = s.fin.wallet.max(0);
    s.modal = Some(Modal::Amount(AmountState {
        title: title.into(),
        body: body.into(),
        min: 0,
        max,
        step: 1_000,
        value: (max / 4 / 1000) * 1000,
        confirm: "Invertir".into(),
        on_ok: Some(Box::new(move |s: &mut State, v| {
            if v > 0 && s.fin.invest(kind, v) {
                s.toast("chart", "Inversión realizada", kind.name(), -v);
                let key = format!("invested_{:?}", kind);
                if !s.flag(&key) {
                    s.set_flag(&key);
                    let (t, txt, sk, sc) = match kind {
                        InvestKind::Deposit => ("Depósito a plazo", "Invertiste en un depósito a plazo: poco riesgo, ganancia pequeña pero segura.", Skill::Risk, 1),
                        InvestKind::Fund => ("Fondo mutuo", "Invertiste en un fondo diversificado: aceptaste algo de riesgo a cambio de una ganancia posible mayor.", Skill::Risk, 1),
                        InvestKind::Crypto => ("MoonCoin", "Invertiste en un activo muy especulativo porque \"se iba a multiplicar\". El riesgo era enorme.", Skill::Risk, -1),
                    };
                    s.fin.journal(t, txt, sk, sc);
                }
            }
            s.modal = Some(Modal::Phone(PhoneApp::Invest));
        })),
        cancel: true,
    }));
}

pub fn on_withdraw(s: &mut State, gain: i64) {
    if gain > 0 && !s.flag("withdraw_gain") {
        s.set_flag("withdraw_gain");
        s.fin.journal(
            "Rescataste con ganancia",
            &format!("Sacaste tu inversión con {} de ganancia. Saber cuándo salir también es parte de invertir.", money(gain)),
            Skill::Risk,
            1,
        );
    }
}

pub fn prepay_debt(s: &mut State, i: usize) {
    if i >= s.fin.debts.len() {
        return;
    }
    let d = &s.fin.debts[i];
    let pay = d.installment.min(d.remaining());
    if s.fin.wallet < pay {
        s.toast("warning", "No te alcanza", "Necesitas dinero en la billetera para adelantar una cuota", 0);
        return;
    }
    s.fin.wallet -= pay;
    let d = &mut s.fin.debts[i];
    if d.overdue > 0 {
        d.overdue = 0;
    } else if d.payments_left > 0 {
        d.payments_left -= 1;
    }
    d.paid += pay;
    s.fin.log("Pago adelantado de cuota", -pay, super::finance::Account::Credit);
    s.fin.debts.retain(|d| d.payments_left > 0 || d.overdue > 0);
    s.toast("check", "Cuota pagada", "Adelantaste una cuota", -pay);
}

pub fn after_scene(s: &mut State) {
    let _ = s;
}

// ------------------------------------------------------------------ debug helpers

pub fn debug_beat(s: &mut State, b: &str) {
    match b {
        "shop" => {
            s.fin.earn(30_000, "debug");
            s.goto(Loc::Plaza, "fountain");
            s.modal = Some(Modal::Shop(ShopState {
                shop: Shop::TecnoMundo,
                selected: Some(0),
                message: None,
                compare: true,
            }));
        }
        "phone" => {
            s.fin.earn(30_000, "Regalo de cumpleaños (Abuela Rosa)");
            s.fin.to_savings(15_000);
            create_goal(s, "trip");
            s.fin.to_goal("trip", 8_000);
            s.fin.take_credit("Zapatillas urbanas", 34_990, 6, 0.04, "tecno");
            s.message("abuela", "¡Feliz cumpleaños, mi niña! Te transferí $30.000.");
            s.modal = Some(Modal::Phone(PhoneApp::Bank));
        }
        "goals" => {
            s.fin.earn(30_000, "debug");
            create_goal(s, "trip");
            create_goal(s, "bike");
            s.fin.to_goal("trip", 20_000);
            s.modal = Some(Modal::Phone(PhoneApp::Goals));
        }
        "choice" => {
            s.goto(Loc::Home, "hall");
            let p = s.chars[0].anim.pos;
            if let Some(m) = s.ch(Who::Mama) {
                m.anim.teleport(p + Vec3::new(-1.1, 0.0, 0.3), 1.3);
            }
            if let Some(steps) = talk(s, Who::Mama) {
                s.dir.run(steps);
            }
        }
        "summary" => {
            s.fin.earn(30_000, "debug");
            s.fin.to_savings(20_000);
            let rep = s.fin.end_week(false);
            s.week = s.fin.week;
            s.modal = Some(Modal::Summary(rep));
        }
        "biz" => {
            s.biz.active = true;
            s.biz.partner = true;
            s.fin.earn(30_000, "debug");
            s.goto(Loc::Plaza, "fountain");
            s.modal = Some(Modal::Business(business::BizUi::new(&s.biz)));
        }
        "cafe" => {
            s.goto(Loc::Plaza, "fountain");
            s.modal = Some(Modal::Minigame(minigame::CafeGame::new(false, 1)));
        }
        "reflect" => {
            s.fin.journal("El regalo de la abuela", "Usaste el regalo para empezar tu meta.", Skill::Planning, 2);
            s.fin.journal("Tu ahorro te salvó", "Pagaste la reparación con tus ahorros.", Skill::Resilience, 2);
            s.fin.journal("Zapatillas en cuotas", "Pagaste $6.000 extra en intereses.", Skill::Credit, -1);
            s.fin.journal("Detectaste una estafa", "Ignoraste el mensaje.", Skill::Risk, 2);
            s.modal = Some(Modal::Reflection);
        }
        "dialog" => {
            s.goto(Loc::Home, "hall");
            let p = s.chars[0].anim.pos;
            if let Some(m) = s.ch(Who::Mama) {
                m.anim.teleport(p + Vec3::new(-1.1, 0.0, 0.0), 1.57);
            }
            s.dir.run(vec![
                face(Who::Sofia, Target::Char(Who::Mama)),
                shot(ShotKind::Over(Who::Sofia, Who::Mama)),
                say_e(Who::Mama, Expr::Happy, "¡Feliz cumpleaños, hija! Quince años... ¿cuándo pasó tan rápido?"),
            ]);
        }
        "week3" => {
            s.week = 3;
            s.fin.week = 3;
            let steps = week_start(s);
            s.dir.run(steps);
        }
        "plaza" => {
            s.goto(Loc::Plaza, "home");
            s.set_phase(Phase::Afternoon);
        }
        "armtest" => {
            use crate::character::anim::{ph, Action, Set};
            let k = s.chars[0].k();
            let a = Action {
                name: "armtest",
                hold: true,
                phases: vec![ph(
                    0.5,
                    crate::math::Ease::InOut,
                    vec![
                        Set::Hand(1, Vec3::new(-0.35, 1.35, 0.25) * k, Vec3::new(0.0, 0.0, 1.0), 1.0),
                        Set::Hand(0, Vec3::new(0.25, 1.0, 0.3) * k, Vec3::new(0.0, 1.0, 0.0), 1.0),
                    ],
                )],
            };
            let p = s.chars[0].anim.pos + Vec3::new(0.0, 0.0, 0.8);
            s.chars[0].anim.teleport(p, 0.0);
            s.chars[0].anim.play(a);
            s.chars[0].held[1] = Some(HeldProp::Phone);
        }
        _ => {}
    }
}
