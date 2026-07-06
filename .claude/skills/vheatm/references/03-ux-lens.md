<!-- VHEATM 10.0: unchanged from 9.0 -->
# User Journey Lens — [G.UX]

**Required when:** Context Mode = DESIGN or Audit Scope = PRE-LAUNCH
**Recommended when:** QBR ≥ 17 hypotheses have user_facing_impact ≥ 2
**Skip when:** Context Mode = LEGACY or scope = BUG HUNT (pure internal)

UX-GAP hypotheses route to [G.H] with user_facing_impact weighted ×2 — UX failures are often invisible in code review.

---

## Step 1: Persona Definition

Choose 3 user personas — specific, not abstract categories:

```
Persona X: [Name, age, context, tech literacy, emotional state]
  Example: "Lan, 24, first-time user, average tech literacy, slightly anxious"

Persona Y: [Name, age, context, tech literacy, emotional state]
  Example: "Minh, 35, power user, high tech literacy, time-pressured"

Persona Z: [Name, age, context, tech literacy, emotional state]
  Example: "Bà Nga, 58, infrequent user, low tech literacy, skeptical of technology"
```

**Rule:** At least one persona must be a first-time or low-expertise user.
**Rule:** At least one persona must represent a potentially vulnerable context (stress, urgency, unfamiliarity).

---

## Step 2: Happy Path Simulation

For each persona, trace the core user journey step by step:

```
Persona: [Name]
Goal: [What they're trying to accomplish]

Step 1: [Action user takes]
  Expected: [What system should do]
  Actual (from design/code): [What system does]
  UX Gap? YES: [describe friction] / NO

Step 2: [Action]
  ...

Overall: Persona reaches goal? YES / NO / PARTIAL
  If NO or PARTIAL → generate UX-GAP hypothesis
```

---

## Step 3: Failure Mode Simulation

For each persona, simulate 2 failure scenarios:

```
Failure scenario: [What goes wrong — network error, wrong input, edge case]
User sees: [Error message / empty state / crash / confusing UI]
User understands what happened? YES / NO / PARTIAL
User knows what to do next? YES / NO / PARTIAL
Emotional impact: Frustrated / Confused / Scared / Abandoned / Neutral
UX-GAP severity: LOW / MEDIUM / HIGH
```

---

## Step 4: Edge Case Simulation

"What does the most vulnerable user do that the average user doesn't?"

```
Edge case: [Describe unusual but plausible user behavior]
System response: [What happens]
Risk: LOW / MEDIUM / HIGH
  — especially HIGH for: safety-critical systems, financial, medical, or crisis contexts
```

---

## UX Lens Output

```
UX-GAP hypotheses generated:
  UX-01: [Description] — QBR: [score, with user_facing_impact ×2]
  UX-02: [Description] — QBR: [score]
  (or: "No UX gaps identified")

Route to [G.H] as type=UX-GAP hypotheses.

Note: UX-GAP findings use Evidence Tier T3-T4 — no file:line expected.
Document: persona name + failure scenario as evidence anchor.
```

---

## Known Limitation

UX Lens is a simulation, not actual user research. It surfaces likely friction points but cannot replace:
- User interviews
- Usability testing with real users
- A/B testing

Document explicitly: "UX findings are simulation-based. Recommend user research validation before treating UX-GAP hypotheses as MANDATORY."
