# Machine Empire — Game Design Document

**Version:** 0.1
**Date:** March 2, 2026
**Genre:** Grand Strategy + Real-Time Tactics
**Players:** 2–4 (Human, AI, Agent — any combination)
**Theme:** Machine civil war — identical factions fighting for dominance over shared territory

---

## 1. Game Concept

Machine Empire is a two-layer strategy game. On the **campaign map**, players manage their empire in pausable real-time: building armies, researching technology, and dispatching forces to contest resource sites scattered across no-man's land. When opposing forces meet at a site, the game drops into a **tactical RTS battle** where the player deploys a command post, explores the site, captures control points, and holds them to win.

The strategic tension lives in resource management. Reinforcements during RTS battles drain the global economy — every unit sent into a contested site is one you can't use elsewhere. Battles can become meatgrinders that bankrupt a player who doesn't know when to cut their losses.

Victory is achieved by destroying every enemy player's main forge.

---

## 2. Two-Layer Structure

```
┌─────────────────────────────────────────────────────────┐
│                    CAMPAIGN MAP                          │
│                 (Pausable Real-Time)                     │
│                                                          │
│   [Player Forge] ──── no-man's land ──── [Enemy Forge]  │
│         │          ⛏ Mining Station           │          │
│         │          🏛 Relic Site               │          │
│         │          ⛏ Mining Station           │          │
│         │          🏛 Relic Site               │          │
│         │          ⛏ Mining Station           │          │
│                                                          │
│   Players dispatch expeditionary forces to contest       │
│   resource sites. When forces arrive → triggers battle   │
└──────────────────────────┬──────────────────────────────┘
                           │
                    Force arrives at site
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                    RTS BATTLE MAP                        │
│                      (Real-Time)                         │
│                                                          │
│   Player deploys Command Post → reinforcements enabled   │
│   Explore site → locate capture points                   │
│   Fight enemy forces → hold capture points → win         │
│                                                          │
│   Reinforcements drain global economy continuously       │
│   Lose command post = no more reinforcements             │
│   Win = control the site on campaign map                 │
└─────────────────────────────────────────────────────────┘
```

### How the Layers Interact

1. **Campaign → RTS:** Player selects units from their global army to form an expeditionary force. They choose a target resource site and dispatch. Travel takes time based on distance. On arrival, if the site is uncontested, they claim it peacefully. If an enemy force is present (or arrives while you're there), an RTS battle begins.

2. **RTS → Campaign:** Battle outcome determines site ownership. Units that survive return to the player's available forces. Units that die are gone permanently — they must be replaced using global production. Resources spent on reinforcements during the battle are deducted from the global economy.

3. **Economy flows both ways:** Controlling mining stations increases global income. That income funds unit production and reinforcements. Losing a site means losing that income stream. Pouring reinforcements into a losing battle can spiral into economic collapse.

---

## 3. Campaign Map

### 3.1 Overview

The campaign map is a strategic-level view showing all players' main forges and the contested resource sites between them. It operates in **pausable real-time** — time flows continuously but any player can pause to issue orders (in multiplayer, pausing may be limited or voted on).

### 3.2 Player Forge (Home Base)

Each player starts with one main forge. This is their home base and the heart of their empire.

**Functions:**
- Produces all unit types (Thralls, Sentinels, Hover Tanks)
- Houses the player's global army reserve
- Generates base resource income
- Hub for research (requires Relic Sites to unlock tech)
- **If destroyed → player is eliminated**

**Properties:**
- Cannot be relocated
- Has substantial defenses (details TBD — may involve a siege RTS battle to attack)
- Visible to all players on the campaign map at all times

### 3.3 Resource Sites

Resource sites are scattered across no-man's land between player forges. They are the primary reason to fight.

#### Mining Stations

- **Provide:** Steady resource income per campaign tick while controlled
- **Quantity:** More numerous, spread across the map
- **Strategic value:** Economic backbone — control more mines, build more units
- **When contested:** Standard RTS battle at the mining site

#### Relic Sites

- **Provide:** Access to technology research (unlocks upgrades and new capabilities)
- **Quantity:** Fewer, more valuable — perhaps 2-4 on a standard map
- **Strategic value:** Qualitative advantage — a player with relics can research upgrades that make their units better, even if they have fewer of them
- **When contested:** Standard RTS battle at the relic site
- **Note:** Controlling a relic site doesn't automatically give tech — it enables research options at the player's forge, which costs resources and time

### 3.4 Campaign Map Flow

```
1. Game starts → each player has a forge + ~500 energy bank + starting army
2. Players dispatch forces to claim nearby uncontested sites (no battle needed)
3. Income grows → fund larger armies and push for contested sites
4. Players must garrison controlled sites or risk losing them to raids
5. Relic sites become high-priority targets for tech advantage
6. Players must balance:
   - Garrisoning held sites (cheap upkeep, but units tied up)
   - Attacking new sites (deployed upkeep burns energy)
   - Reinforcing ongoing battles (economic drain from upkeep + strain)
   - Upgrading forge production lines (invest now, pay off later)
   - Saving for research (requires relic control + energy + time)
   - Keeping a mobile reserve to respond to enemy raids
7. Late game: players with economic/tech advantage assault enemy forges
8. Last forge standing wins
```

### 3.5 Campaign Actions

| Action | Description |
|--------|-------------|
| **Produce Units** | Queue unit production at forge. Costs energy, takes time, uses a production line |
| **Upgrade Forge** | Add production lines (infantry or armor). Costs energy + time |
| **Dispatch Force** | Select units from reserve, assign to a target site. Travel time based on distance |
| **Reinforce Battle** | Send additional units from reserve to an ongoing RTS battle (if command post is active) |
| **Withdraw** | Recall surviving units from a battle or garrisoned site back to reserve |
| **Garrison** | Assign units to defend a controlled site. Switches them to cheaper garrisoned upkeep |
| **Research** | Spend energy + time to unlock tech (requires controlling a relic site) |
| **Pause/Unpause** | Pause campaign time to assess and issue orders |

---

## 4. RTS Battles

### 4.1 Battle Setup

When a player's expeditionary force arrives at a resource site and an enemy is present (or arrives), an RTS battle begins.

**Deployment phase:**
1. Each player's force arrives at an entry zone on their side of the battle map
2. Player places their **Command Post** within the entry zone
3. Initial forces deploy around the command post
4. Battle begins — fog of war is active, site must be explored

### 4.2 Command Post

The command post is the **only building** in the RTS layer. It serves critical functions:

- **Reinforcement beacon:** While the command post is alive, the player can call in reinforcements from their global army reserve. Units arrive at the command post after a short delay.
- **Provides vision:** Has a large sight radius around itself
- **Destructible:** If destroyed, no more reinforcements can arrive for the rest of the battle. Existing units continue to fight.
- **One per player per battle:** Cannot be rebuilt

**Properties (initial balance — subject to tuning):**

| Stat | Value |
|------|-------|
| Health | High (takes sustained effort to destroy) |
| Armor | Moderate |
| Vision range | Large |
| Reinforcement delay | Time between requesting reinforcements and units arriving |
| Build time | Instant (placed during deployment phase) |

### 4.3 Capture Points

Each RTS battle map has **capture points** that must be controlled to win.

**Mechanics:**
- Capture points are marked locations on the map (visible once explored through fog of war)
- A player captures a point by moving units near it with no enemy units contesting it
- Capture takes time — a progress bar fills over several seconds
- If enemy units arrive during capture, the progress pauses (contested)
- Once captured, the point contributes to that player's control score
- A captured point can be re-captured by the enemy through the same process

**Win condition:**
- Control **all** capture points simultaneously, OR
- Hold a **majority** of capture points for a sustained duration (e.g., 60 seconds of majority control), OR
- Destroy all enemy units AND their command post (total elimination)

The sustained-hold condition prevents stalemates where both sides endlessly trade points.

### 4.4 Reinforcements (The Meatgrinder)

This is the core economic mechanic of Machine Empire.

**How it works:**
1. During an RTS battle, if the player's command post is active, they can request reinforcements
2. Reinforcements are drawn from the player's **global army reserve** — these are units that were already produced on the campaign map
3. If the reserve is empty, the player can queue new production at their forge — but this costs energy and takes production time before the units are available, then additional travel/arrival time
4. Each reinforcement unit that arrives and dies in battle is **permanently lost**
5. All units in the battle incur **deployed upkeep** — the longer the battle drags on, the more energy it costs
6. Thrall reinforcements add **Conscription Strain** if produced during the battle (see Section 6.6)

**The strategic tension:**
- Winning a contested site means economic advantage (mining income or tech access)
- But every unit in that battle is costing deployed upkeep every second
- Thrall replacements spike Conscription Strain, slowing your entire economy
- A savvy player recognizes when a battle is unwinnable and **withdraws** to preserve their remaining forces and stop the upkeep bleed
- An aggressive player can bleed an opponent dry by forcing them into multiple simultaneous battles — each one burning deployed upkeep

**Withdrawal:**
- Players can order a **retreat** at any time during an RTS battle
- Surviving units disengage and return to the global army reserve after a delay
- The command post is abandoned (destroyed/lost)
- Retreating under fire incurs casualties (units can be killed while withdrawing)
- Surviving units switch back to garrisoned upkeep rate once they return to reserve

### 4.5 Fog of War

- Full fog of war in all RTS battles
- Each unit has a vision radius
- Unexplored areas are black (never seen)
- Explored areas show terrain but not current enemy positions (dimmed)
- Visible areas show everything in real-time
- Capture points are not visible until explored — players must scout to find them

---

## 5. Units

All factions use identical units — this is a civil war. Differentiation comes from tech upgrades, strategic decisions, and player skill.

### 5.1 Unit Cost Philosophy

Units fall into two fundamentally different economic categories:

**Conscripted units (Thralls):** Cheap energy cost and fast to produce, BUT each Thrall is pulled from the civilian population. Mass conscription causes **Conscription Strain** — an escalating economic penalty that compounds the faster you conscript. See Section 6 for the full strain system.

**Manufactured units (Sentinels, Hover Tanks):** These are purpose-built war machines. They cost more energy and take longer to produce, but have **zero population impact**. Building 50 Hover Tanks doesn't hurt your economy beyond the energy spent. The constraint is purely cost and production time.

This creates a core strategic tension: Thralls are the fast, cheap option that lets you respond to crises quickly — but over-reliance on Thralls (especially panic-replacing heavy losses) can cripple your economy through strain. Sentinels and Hover Tanks are the sustainable long-term investment — expensive upfront but no hidden costs.

### 5.2 Thralls

*Conscripted infantry. Cheap, fast, expendable — but every one pulled from the population has a cost beyond energy.*

| Stat | Value | Notes |
|------|-------|-------|
| Role | Basic infantry | Expendable frontline |
| Health | Low | Dies quickly under focus fire |
| Damage | Low | Strength is in numbers |
| Speed | Fast | Quick to reposition |
| Range | Medium | Standard infantry engagement range |
| Energy cost | Very low | |
| Production time | Very fast | |
| Population impact | **Yes — adds Conscription Strain** | See Section 6 |

**Design intent:** Thralls are the bread and butter — fast to produce, cheap in energy, and the first units available for scouting, skirmishing, and early expansion. A swarm of Thralls can overwhelm a smaller elite force, but they melt against Hover Tanks. The critical hidden cost is Conscription Strain: maintaining a steady trickle of Thralls is fine, but panic-replacing 30 dead Thralls after a lost battle will spike strain and crater your economy. The player who uses Thralls wisely — accepting losses rather than immediately replacing them — has a significant economic advantage.

**Future:** Upgradeable via tech research (e.g., improved armor, faster fire rate, longer range).

### 5.3 Sentinels

*Elite cyborg infantry. Purpose-built for war. Expensive but no population impact.*

| Stat | Value | Notes |
|------|-------|-------|
| Role | Elite infantry | Holds ground, punches above weight |
| Health | High | Significantly tougher than Thralls |
| Damage | High | Effective against all unit types |
| Speed | Medium | Slower than Thralls |
| Range | Medium | Same engagement range as Thralls |
| Energy cost | High | |
| Production time | Slow | |
| Population impact | **None** | Manufactured unit |

**Design intent:** Sentinels are the quality answer to Thrall quantity. A squad of Sentinels can hold a capture point against a much larger Thrall force. They're the sustainable fighting force — losing them hurts because of the energy and time invested, but replacing them doesn't damage your economy further through strain. They're the core of a mid-to-late game strategy and the backbone of a player who wants to avoid the Thrall conscription trap.

**Future:** Upgradeable via tech research (e.g., shield generators, heavy weapons, stealth).

### 5.4 Hover Tanks

*Heavy armored vehicles. Manufactured war machines. Devastating but costly.*

| Stat | Value | Notes |
|------|-------|-------|
| Role | Heavy armor | Area denial, assault breaker |
| Health | Very high | Takes a lot of firepower to bring down |
| Damage | Very high | Can shred infantry groups |
| Speed | Medium-fast | Hover movement ignores terrain penalties |
| Range | Long | Outranges infantry |
| Energy cost | Very high | |
| Production time | Very slow | |
| Population impact | **None** | Manufactured unit |

**Design intent:** Hover Tanks are the endgame power unit. A player who reaches late-game with a healthy economy and invests in Hover Tanks has a massive tactical advantage. But they're so expensive in energy that losing one is devastating — especially if the opponent is bleeding you with cheap Thrall waves while your economy is tied up in tank production. The hover mechanic (ignoring terrain cost) makes them uniquely mobile on rough terrain, allowing flanking maneuvers that infantry can't match.

**Future:** Upgradeable via tech research (e.g., siege mode, anti-air, improved armor plating).

### 5.5 Unit Interaction Philosophy

There is **no hard counter system** (rock-paper-scissors). Instead, the balance comes from economics, strain management, and positioning:

- **Thralls vs Thralls:** Numbers win. Whoever has more and positions better. But the loser faces a strain spike replacing casualties.
- **Thralls vs Sentinels:** Sentinels win straight fights, but Thralls are cheaper in energy. If you trade 5 Thralls to kill 1 Sentinel, that's energy-efficient for the Thrall player — but the strain cost of replacing 5 Thralls quickly makes it less clear-cut.
- **Thralls vs Hover Tanks:** Hover Tanks dominate. Thralls need overwhelming numbers or terrain advantage. Losing masses of Thralls to tanks and panic-replacing them is the classic meatgrinder trap.
- **Sentinels vs Hover Tanks:** Sentinels can fight tanks but it's a losing trade in energy. Sentinels are better used against infantry.
- **Hover Tanks vs Hover Tanks:** Micro and positioning matter. First shot advantage is significant.
- **Mixed forces:** The strongest armies combine all three. Thralls scout and screen (accepting some are disposable), Sentinels hold capture points, Tanks break through contested positions.

---

## 6. Economy & Conscription Strain

### 6.1 Core Economic Model

Energy is the single resource. It flows continuously as a rate (energy per second), not in chunks. The player's **net energy rate** is:

```
Net Energy/sec = (Forge Base Income + Mining Income + Relic Income)
               × (1 - Strain Income Penalty)
               - Unit Upkeep (garrisoned + deployed)
               - Active Production Costs
               - Active Research Costs
```

The player also has an **energy bank** — a stored pool of energy that absorbs temporary deficits. If net rate goes negative, the bank drains. If the bank hits zero with a negative rate, production and research pause until income recovers.

### 6.2 Income Sources

| Source | Base Rate | Notes |
|--------|-----------|-------|
| Forge | ~5 energy/sec | Always active. Enough to slowly trickle-produce Thralls |
| Mining Station | ~8 energy/sec per site | Main economic driver. All mines equal at start, upgradeable later |
| Relic Site | ~3 energy/sec per site | Small income; real value is enabling research |

**Early game example (1 forge + 2 mines):**
```
Income = 5 + 8 + 8 = 21 energy/sec
```

**Mid game example (1 forge + 4 mines + 1 relic):**
```
Income = 5 + 32 + 3 = 40 energy/sec
```

*All numbers are placeholder — will be tuned through AI vs AI playtesting.*

### 6.3 Expenses

#### Unit Production Costs

Production has two costs: an **energy price** (deducted from bank on completion) and **production time** (occupies a production line).

| Unit | Energy Cost | Production Time | Production Line |
|------|------------|----------------|-----------------|
| Thrall | ~30 energy | ~5 sec | Infantry line |
| Sentinel | ~120 energy | ~15 sec | Infantry line |
| Hover Tank | ~300 energy | ~30 sec | Armor line |

#### Unit Upkeep

All units cost ongoing energy. Deployed units (in transit, in battle, on expedition) cost more than garrisoned units (sitting in reserve at the forge or at a controlled site).

| Unit | Garrisoned Upkeep | Deployed Upkeep | Notes |
|------|------------------|-----------------|-------|
| Thrall | ~0.1 energy/sec | ~0.3 energy/sec | Cheap individually, adds up in swarms |
| Sentinel | ~0.3 energy/sec | ~0.8 energy/sec | Noticeable per-unit cost |
| Hover Tank | ~0.8 energy/sec | ~2.0 energy/sec | Expensive to keep in the field |

**Why upkeep matters:** It creates a natural army size ceiling. A player with 40 energy/sec income can't sustain a 200-Thrall army (200 × 0.3 = 60 energy/sec deployed upkeep alone). This replaces a hard population cap with an economic soft cap — you CAN build a huge army, but you'll go bankrupt maintaining it.

**Garrison incentive:** Garrisoned upkeep is roughly 1/3 of deployed upkeep. This rewards keeping reserves at home rather than having your entire army in the field. It also means controlling a site with a small garrison is cheap, but deploying a massive expeditionary force is expensive every second it's out.

#### Upkeep Budget Example

```
Mid-game army: 30 Thralls + 8 Sentinels + 3 Hover Tanks

If all garrisoned:
  30 × 0.1 + 8 × 0.3 + 3 × 0.8 = 3.0 + 2.4 + 2.4 = 7.8 energy/sec
  With 40 energy/sec income → 32.2 energy/sec left for production & research ✓

If all deployed:
  30 × 0.3 + 8 × 0.8 + 3 × 2.0 = 9.0 + 6.4 + 6.0 = 21.4 energy/sec
  With 40 energy/sec income → 18.6 energy/sec left — tight but functional

If army doubles to 60 Thralls + 16 Sentinels + 6 Tanks (all deployed):
  60 × 0.3 + 16 × 0.8 + 6 × 2.0 = 18.0 + 12.8 + 12.0 = 42.8 energy/sec
  With 40 energy/sec income → -2.8 energy/sec → DEFICIT, bank draining
  Need more mining stations or must garrison some forces
```

This naturally creates the "comfortable early, tight mid/late" feel. Early on you have a small army and income covers everything easily. As the army grows and you deploy more forces, upkeep eats into income and every decision about where to send troops matters.

#### Research Costs

Research costs energy and time. Costs scale — early techs are cheap, later techs are expensive.

| Tech Tier | Energy Cost | Research Time | Requires |
|-----------|------------|---------------|----------|
| Tier 1 | ~200 energy | ~60 sec | 1 Relic Site |
| Tier 2 | ~500 energy | ~120 sec | 1 Relic Site + Tier 1 prerequisite |
| Tier 3 | ~1000 energy | ~180 sec | 2 Relic Sites + Tier 2 prerequisite |

Research consumes energy from the bank as a lump sum when initiated (not as a rate). If you can't afford it, you wait until the bank fills enough. Only one research project active at a time (to start — could be expanded via forge upgrade later).

#### Forge Upgrades

The forge can be upgraded to expand production capacity. These are the initial upgrade paths:

| Upgrade | Cost | Effect |
|---------|------|--------|
| Additional Infantry Line | ~400 energy + ~45 sec | Can produce 2 infantry units simultaneously |
| Additional Armor Line | ~600 energy + ~60 sec | Can produce 2 armor units simultaneously |
| 3rd Infantry Line | ~800 energy + ~60 sec | 3 simultaneous infantry |
| 3rd Armor Line | ~1200 energy + ~90 sec | 3 simultaneous armor |

*The system is built to be extensible — future forge upgrades (better base income, improved defenses, faster research, etc.) can be added without restructuring the economy.*

**Starting production capacity:**
- 1 Infantry line (produces Thralls or Sentinels, one at a time)
- 1 Armor line (produces Hover Tanks, one at a time)

Each additional line allows one more unit of that category to be in production simultaneously. A player with 3 infantry lines can build 3 Thralls at once, or 2 Thralls and 1 Sentinel, etc.

### 6.4 Starting Conditions

Each player begins with:

| Asset | Amount |
|-------|--------|
| Energy bank | ~500 energy |
| Forge | 1 (with 1 infantry line + 1 armor line) |
| Forge base income | ~5 energy/sec |
| Starting Thralls | ~10 |
| Starting Sentinels | ~3 |
| Starting Hover Tanks | ~1 |
| Mining Stations | 0 (must be claimed) |
| Relic Sites | 0 (must be claimed) |

The starting bank + income is enough to immediately dispatch a small expeditionary force to claim the nearest mining station without a battle (if no enemy contests it). From there, income ramps up and the player begins real production.

### 6.5 Site Garrison & Defense

**Undefended sites can be claimed without a battle.** If a player sends forces to an ungarrisoned enemy mining station, they simply take control. No RTS battle — just ownership transfer after travel time.

**Defended sites require an RTS battle.** If the site has garrisoned units, the attacker must fight for it.

This creates garrison management as a core strategic skill:

```
You control 5 mining stations.
You have 40 Thralls + 10 Sentinels.

Option A: Garrison 3-5 units at each site (15-25 units tied up in defense)
  → Sites are protected from raids
  → Fewer units available for attack
  → Garrisoned upkeep is cheap (0.1-0.3 per unit)

Option B: Leave sites ungarrisoned, keep full army as strike force
  → Maximum offensive power
  → Enemy can steal undefended sites without fighting
  → Must react quickly to defend (dispatch units, incur deployed upkeep)

Option C: Garrison critical sites (rich mines, relics), leave others exposed
  → Balanced approach — accept losing cheap sites if attacked
  → Keep enough mobile force to counterattack
```

**Garrison swapping:** When an enemy force approaches a site you control, you receive an alert and have time (based on enemy travel distance) to either reinforce the garrison or withdraw it. Withdrawing concedes the site without a fight.

### 6.6 Conscription Strain

The signature economic mechanic of Machine Empire. Thralls are conscripted from the civilian population — every Thrall pulled from the workforce adds **strain** to the empire. Strain is **rate-based**: it's not how many Thralls you have, it's how fast you're conscripting them.

#### The Strain Meter

```
Strain: 0 (healthy) ──────────────────────────► 100 (crisis)

Each Thrall conscripted:  +N strain (fixed value per Thrall, tunable)

Recovery per second:
  recovery_rate = BASE_DECAY × (1 - strain/100)²

  This is a squared curve — the deeper the hole, the slower the recovery.
```

#### Recovery Examples

```
Strain 20:  recovery = base × 0.64  → fades in seconds
Strain 40:  recovery = base × 0.36  → fades in ~1 minute  
Strain 60:  recovery = base × 0.16  → takes several minutes
Strain 80:  recovery = base × 0.04  → takes 5-10 minutes
Strain 95:  recovery = base × 0.0025 → potentially game-crippling
```

The squared curve is what makes it compound. Doubling the conscription doesn't just double recovery time — it roughly **quadruples** it or worse. This is intentional: moderate use of Thralls is completely sustainable, but panic-replacing massive losses creates a deep economic wound.

#### Strain Thresholds & Effects

| Strain | Income Penalty | Production Speed | Recovery Feel |
|--------|---------------|-----------------|---------------|
| 0–30 | None | Normal | Fades in seconds |
| 30–50 | -5% to -15% | -10% slower | Fades in ~1 minute |
| 50–70 | -15% to -30% | -25% slower | Takes several minutes |
| 70–90 | -30% to -50% | -50% slower | Takes 5–10 minutes |
| 90+ | -50%+ | Near-halted | Potentially game-crippling |

Both income AND production speed are affected. High strain means less energy coming in AND everything takes longer to build — including Sentinels and Hover Tanks. This means over-conscripting Thralls doesn't just hurt Thrall production; it slows your entire war machine.

#### Scenario: The Meatgrinder Trap

```
Player A sends 40 Thralls to contest a mining station.
Battle goes badly — 30 Thralls die.

Option 1: Accept the loss, retreat with 10 survivors.
  → Strain barely moves. Economy intact.
  → Rebuild slowly over next few minutes.

Option 2: Panic — immediately conscript 30 replacement Thralls.
  → Strain spikes from 10 to 70.
  → Income drops 30-50%, production slows 50%.
  → Recovery takes 5-10 minutes.
  → During that window, opponent with healthy strain can:
     - Outproduce you in Sentinels and Tanks
     - Contest your other mining stations
     - Push for relic sites while you're crippled
  → You've turned a lost battle into a lost game.

Option 3: Conscript 10 replacements now, 10 more in 2 minutes, 10 more later.
  → Strain rises to ~30, recovers between waves.
  → Manageable penalty. Slower to recover force but economy survives.
  → This is the correct play — disciplined conscription over time.
```

#### Strain Visibility

Conscription Strain is **private** to each player. Opponents cannot see your strain level directly. Experienced players can infer it from behavior (sudden shift to defensive play, reduced reinforcement flow, switching to Sentinel-heavy composition). Future feature: espionage or intel mechanics may allow revealing an opponent's strain level.

#### Key Design Notes

- Strain is rate-based, not total-based. Having 100 active Thralls doesn't cause strain. Conscripting 30 Thralls in one minute does.
- Only Thrall production causes strain. Sentinels and Hover Tanks are manufactured — zero strain.
- Strain affects the ENTIRE economy (income + production speed), not just Thrall production. This prevents players from simply switching to Sentinel production during high strain — everything is slower.
- Dead Thralls returning population is handled implicitly by the natural strain decay. The abstraction is "population recovers over time" rather than tracking individual civilians.
- All numbers (strain per Thrall, base decay rate, threshold percentages) are tunable constants. Exact balance will come from AI vs AI playtesting.

### 6.7 Economic Death Spiral

The game is designed so that **losing sites creates a negative feedback loop** that can be hard to recover from:

1. Lose a mining station → income drops
2. Lower income → upkeep eats a larger share → less left for production
3. Fewer new units → harder to contest remaining sites
4. Opponent gains more sites → their income rises → they produce faster
5. Eventually the resource-starved player can't even maintain their existing army

Conscription Strain adds a second spiral on top of this:

1. Lose Thralls in a bad battle → panic-conscript replacements
2. Strain spikes → income drops further, production slows
3. Reduced production → can't build Sentinels/Tanks to stabilize
4. Forced to keep using cheap Thralls → more losses → more strain
5. Economy collapses from both lost income AND high strain

Upkeep adds a third pressure:

1. Large army deployed across multiple fronts → high deployed upkeep
2. Upkeep exceeds income → bank starts draining
3. Must either withdraw forces (ceding territory) or go bankrupt
4. Bankruptcy pauses all production and research → military stagnation

These three spirals (income loss + strain + upkeep) are intentional and interlock. They reward aggressive early expansion, efficient battles, disciplined conscription, and the wisdom to retreat and consolidate rather than overextend.

### 6.8 Economic Pacing Summary

```
EARLY GAME (0-5 min):
  Income: 5 (forge) → 21 (forge + 2 mines)
  Army: ~14 units, mostly garrisoned upkeep
  Net rate: strongly positive. Bank growing.
  Feel: Comfortable. Plenty of energy for expansion.

MID GAME (5-15 min):
  Income: 40+ (forge + 4-5 mines + relics)
  Army: 40-60 units, many deployed
  Upkeep: 15-25 energy/sec
  Research: ongoing (lump sum drains from bank)
  Production: 1-2 lines active continuously
  Net rate: thin positive or break-even.
  Feel: Tight. Must choose between more units, research, or saving.

LATE GAME (15+ min):
  Income: 50-60+ (5-7 sites)
  Army: 60-100+ units, heavy deployment
  Upkeep: 30-50 energy/sec
  Hover Tanks eating budget (2.0/sec deployed each)
  Net rate: near-zero or negative during offensives.
  Feel: Every decision matters. Losing a mine is devastating.
         Launching an assault means accepting the upkeep burn.
```

---

## 7. Technology & Relic Sites

### 7.1 Overview

Relic sites are the gateway to technology. Controlling a relic site doesn't grant tech automatically — it **unlocks research options** at the player's forge.

### 7.2 Research Flow

```
1. Capture and hold a Relic Site (via RTS battle)
2. Relic Site control unlocks new research options at the forge
3. Player spends Energy + Time to research a technology
4. Technology applies globally to all current and future units of that type
5. Losing the Relic Site does NOT remove already-researched tech
   (but prevents researching new tech from that site's tree)
```

### 7.3 Tech Design (Placeholder — To Be Expanded)

Technology will be organized by the relic site that enables it. Each relic site on a map could unlock a different branch of upgrades. Possible directions:

**Thrall Upgrades:**
- Improved plating (more health)
- Accelerated fire rate
- Extended range
- Rapid deployment (faster reinforcement arrival)

**Sentinel Upgrades:**
- Shield generators (regenerating shields)
- Heavy weapons package (more damage)
- Stealth systems (reduced detection range)
- Fortification mode (increased defense when stationary)

**Hover Tank Upgrades:**
- Siege mode (increased range + damage when stationary)
- Reactive armor (damage reduction)
- Overcharge engines (speed burst ability)
- Anti-infantry payload (area-of-effect damage)

**Global Upgrades:**
- Increased population cap
- Faster production speed
- Improved forge defenses
- Reduced reinforcement delivery time
- Enhanced fog of war vision range

*Note: Specific tech trees will be designed once core gameplay is functional. The system is built to be extensible.*

---

## 8. Beliefs System (Future)

*Not implemented in initial release. Documented for design intent.*

Beliefs are global modifiers that the player chooses at the start of a game (or unlocks during campaign). They represent the faction's ideology within the machine civil war and provide passive bonuses that shape playstyle.

**Example concepts:**
- **The Swarm Doctrine:** +20% Thrall production speed, -10% Sentinel/Tank health
- **The Iron Citadel:** +30% building defense, -15% unit speed
- **The Efficiency Protocol:** -15% all production costs, -10% all unit health
- **The War Machine:** +15% all damage, +20% all production costs

*Beliefs create asymmetry between identical factions without adding different unit rosters.*

---

## 9. Game Flow (Full Match)

### Early Game (0-5 minutes campaign time)

```
- Each player starts with forge + ~500 energy bank + starting force
  (approximately 10 Thralls, 3 Sentinels, 1 Hover Tank)
- Forge provides 5 energy/sec base income + 1 infantry line + 1 armor line
- Players immediately dispatch forces to claim nearest uncontested mining sites
- Undefended sites are claimed automatically — no battle needed
- First contact usually happens at centrally-located mining sites
- Early battles are small-scale: Thrall-heavy skirmishes
- Priority: secure 2-3 mining stations for income, garrison them lightly
- Smart players balance garrison duty vs mobile strike force
```

### Mid Game (5-15 minutes campaign time)

```
- Players have established income from mining sites
- Relic sites become contested — tech advantage is crucial
- Army composition shifts toward mixed forces (Thralls + Sentinels)
- Smart players begin transitioning to Sentinels to avoid strain dependency
- First Hover Tanks may appear for players with strong economies
- Strategic decisions:
  - Push for more sites or consolidate?
  - Invest in tech or more units?
  - Attack opponent's mining sites to hurt their economy?
  - How aggressively to conscript Thralls vs invest in manufactured units?
- Meatgrinder battles begin at key contested sites
- Players who over-conscript Thralls start feeling strain penalties
```

### Late Game (15+ minutes campaign time)

```
- Clear economic leaders emerge based on site control
- Tech upgrades create qualitative differences between factions
- Hover Tanks become a major factor
- Players with healthy economies and low strain can field mixed armies
- Players trapped in strain spirals are visibly struggling — fewer reinforcements,
  slower production, forced into defensive posture
- Endgame push: assault on enemy forges
- Forge assaults are high-stakes: losing your attack force AND
  leaving your own forge exposed is game-ending
- Victory: last forge standing
```

---

## 10. AI & Agent Design Considerations

### Built-in AI (Difficulty Levels)

The AI plays by the same rules as human players — no cheating (unless configured).

**Campaign layer AI needs:**
- Economic planning (when to build, what to build)
- Site prioritization (which sites to attack, which to defend)
- Reinforcement decisions (when to pour in, when to retreat)
- Scouting and map awareness

**RTS battle AI needs:**
- Unit micro (movement, targeting, retreating wounded units)
- Capture point prioritization
- Command post protection
- Reinforcement timing
- Retreat decision-making

### MCP Agent Interface

AI agents (Claude, etc.) connect via MCP and interact at the **campaign level** and/or the **RTS level**:

**Campaign tools:**
- View map state (sites, ownership, forces)
- Produce units at forge
- Dispatch forces to sites
- Send reinforcements to ongoing battles
- Research technology
- Withdraw forces

**RTS battle tools:**
- View battle state (units, fog of war, capture points)
- Move units
- Attack targets
- Deploy command post
- Request reinforcements
- Retreat

An agent can play fully autonomously (both layers) or be an advisor (suggesting moves to a human player).

---

## 11. Map Design Principles

### Campaign Map

- **Symmetrical start positions** for fairness (2-player: mirrored, 3-4 player: rotational)
- **Mining stations** placed in concentric rings — closer = easier to claim, further = more contested
- **Relic sites** placed centrally or at strategic chokepoints — always contested
- **Distance matters** — dispatching forces to far sites takes longer, giving defenders time to prepare
- **No-man's land** should have natural bottlenecks and open areas, creating strategic terrain

### RTS Battle Maps

- **Generated per site type** — mining sites look different from relic sites
- **3-5 capture points** per map (odd number to prevent permanent ties)
- **Fog of war** — capture points are hidden until explored
- **Entry zones** on opposite sides for each player
- **Terrain variety** — open areas favor Hover Tanks, tight areas favor infantry
- **Size** — small enough for battles to resolve in 5-10 minutes of real-time

---

## 12. Key Design Pillars

1. **Every decision has a cost.** Producing units, attacking sites, reinforcing battles, researching tech — everything costs energy. Thralls add the hidden cost of Conscription Strain. There is never a free choice.

2. **Knowing when to retreat is as important as knowing when to attack.** The meatgrinder mechanic punishes players who throw good money after bad. Retreat preserves your forces AND your economy. The best players lose battles gracefully.

3. **Quantity vs quality is a real choice, with real consequences.** Thrall spam is viable but risks Conscription Strain spirals. Elite Sentinel/Tank armies are sustainable but slow to build. The question is always "can I afford to lose what I'm sending — and can I afford to replace it?"

4. **Map control is everything.** Mining stations fund your war machine. Relic sites upgrade it. Losing either is a step toward defeat.

5. **Identical factions, different strategies.** Since all players have the same units, victory comes from better decisions, not better units. Tech upgrades and (eventually) beliefs create differentiation within a match.

6. **Accessible to AI agents.** The game is designed so that every decision can be made through the MCP interface. An AI agent can play a full game — both campaign and RTS layers — through tool calls alone.

7. **The strain system rewards discipline.** The difference between a good player and a great player is strain management. Knowing how fast to conscript, when to switch to manufactured units, and when to let strain recover is a skill that separates strategic thinkers from reactive players.
