<!-- VHEATM 10.0: unchanged from 9.0 -->
# Fix Verification — [T.FV]

> **Tikai Evidence**: BUG-NH4 v1 fix only updated line 124 (AI input) but left line 133 (DB save) hardcoded to `Decimal("0")`. Audit docstring claimed "FIX BUG-NH4 OK". Bug survived 2 review rounds. Similarly, BUG-H4 (_resolve_entity_id) was documented as fixed but code change was never applied.

Run AFTER any ADR is applied in [T], BEFORE marking it closed.

---

## The Core Trap

**"Documented = Verified" trap.**

In a long audit chain, the same auditor:
1. Identifies bug (correctly)
2. Writes fix description (correctly)
3. Applies fix (PARTIALLY — misses sibling location 9 lines down)
4. Re-reads own documentation (says "fixed")
5. Marks ADR closed

Step 4 is where the trap closes. Auditor's brain reads `# FIX BUG-NH4` comment and believes the fix is complete. **The brain does not re-read the actual code surrounding it.**

Fix Verification breaks this by mandating a code re-read with specific anchor requirements.

---

## Protocol — 3-Anchor Rule

Every ADR must accumulate 3 anchors through its lifecycle:

```
1. Bug Anchor (at [G.H], confirmed in [E]):
   "Bug exists at file:line"

2. Fix Anchor (at [T] after applying):
   "Fix applied at file:line"  ← MUST be different from Bug Anchor unless single-line fix

3. Verification Anchor (at [T.FV] — the new step):
   "Verified by re-reading file:line. Asserts:
    - Original anti-pattern absent
    - New correct pattern present
    - No sibling instances within ±20 lines that share the bug class"
```

---

## Mandatory Steps

### Step 1: Re-read the Fix Location

```
□ Open the file at the Fix Anchor (file:line)
□ Read at least ±20 lines around the change (NOT just the changed line)
□ Look for:
   a) Did the anti-pattern code get removed/changed correctly?
   b) Did the new correct pattern replace it?
   c) Are there OTHER instances of the anti-pattern within the same function/scope?
```

### Step 2: Cross-Reference Bug Anchor vs Fix Anchor

```
Bug Anchor: [file:line of original bug]
Fix Anchor: [file:line of code change]

Match check:
  □ Same file?
  □ Line numbers consistent (accounting for line shifts from the fix itself)?
  □ Function/class context matches?

If ANY answer is NO → investigate: did fix touch the right location?
```

### Step 3: Sibling Scan Within Scope

This is where Tikai F-1 should have caught:

```
Within the same function/method as the fix:
  □ Search for other occurrences of the bug-class pattern
  □ Example: if you fixed `total_estimated_saved=Decimal("0")` at one line,
    grep within the function for other `Decimal("0")` literals near
    semantically-similar fields
```

This is a smaller-scope version of [G.PG] Pattern Globalization, focused on the IMMEDIATE neighborhood of the fix.

### Step 4: Test-as-Proof (when applicable)

If a test was added in [T] Red-Green Gate:

```
□ Run the specific test that proves the fix
□ Confirm GREEN
□ Confirm the test would have FAILED before the fix (red-green discipline)
```

If no test possible (e.g., DESIGN mode, infrastructure change):

```
□ Document why test isn't applicable
□ Add a "Verification by Construction" note explaining how the fix is verified
```

---

## Common Trap Patterns

### Trap T-1: Comment-Driven Verification

```python
# BUG: total_estimated_saved was hardcoded to 0
total_estimated = calculate_estimated_savings(actions)  # fixed
# ... 9 lines later ...
receipt = WeeklyReceipt(
    total_estimated_saved=Decimal("0"),  # ← STILL HARDCODED, not fixed!
)
```

The auditor's eye reads `# fixed` and skips ahead. Fix Verification mandates reading ±20 lines.

### Trap T-2: Half-Documented Fix

ADR says "Fix applied to imports.py to make ARQ pool singleton."
- imports.py: actually fixed ✅
- actions.py: never touched, same bug remains ❌

Without [G.PG] + Fix Anchor cross-check, this gap is invisible.

### Trap T-3: Comment-Only Fix

```python
# FIX BUG-H4: use source_insight_json.entity directly
def _resolve_entity_id(action, snapshot):
    triggers = snapshot.action_triggers_json or []
    for trigger in triggers:                           # ← still first-match!
        if trigger.get("rule_id") == action.rule_trigger:
            return str(trigger.get("entity_id", ""))   # ← unchanged
```

Comment says "fixed", code says "unchanged". Self-audit reads comment, trusts it.

**The fix**: Verification Anchor MUST quote the actual code at fix location, not the comment.

---

## Output Template

```yaml
fix_verification:
  adr_id: ADR-[N]
  bug_anchor: "[file:line]"
  fix_anchor: "[file:line]"
  verification_anchor: "[file:line]"
  re_read:
    range_inspected: "[file:start-end]"  # actual byte range read
    anti_pattern_absent: true | false
    correct_pattern_present: true | false
    sibling_instances_found: 0 | [count]
    sibling_actions: ["fix-id" or "deferred-debt-id" for each]
  test_proof:
    test_name: "[test_func]"
    test_status: GREEN | RED | N/A
    red_green_verified: true | false | N/A
  verdict: VERIFIED | INCOMPLETE | REOPENED
  notes: "..."
```

---

## Failure Modes of Fix Verification Itself

🚫 **Reading the same line you fixed**: If your Verification Anchor matches your Fix Anchor exactly, you haven't verified anything new. Verification REQUIRES reading more context.

🚫 **Trusting your edit tool's diff output**: The tool shows what changed, not whether the change is correct. Read the resulting code as if you're seeing it fresh.

🚫 **Skipping when "obvious"**: If the fix is genuinely 1 line, still re-read ±10 lines. The trap thrives on "this one is obviously correct."

---

## FAST Mode Adaptation

```
In FAST mode, T.FV reduces to:
  □ Re-read fix location ±10 lines (not 20)
  □ Sibling scan: visual only, within same function
  □ No formal test required, but state "would fail before fix" inline

Document: "T.FV-FAST: ±10 line review. Full sibling scan deferred."
```

---

## Integration with [KB]

After each cycle, log to Bug Class Catalog:

```
| Cycle | ADR ID | Bug Anchor | Fix Anchor | Verification Status | Notes |
|-------|--------|-----------|------------|---------------------|-------|
```

If `Verification Status = INCOMPLETE` for ≥2 cycles → escalate the ADR back to [G.H] in next cycle.

---

*Reference 09 — VHEATM 9.0 | Derived from Tikai F-1, F-3 failure modes*
