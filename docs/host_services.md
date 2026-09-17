# DataModel Standard: host services

Written 2026-09-04, after milestone 2's sprints 7 and 8. Hand-written, and
deliberately not part of [`datamodel_scope.md`](datamodel_scope.md), which is
generated.

**A CONFORMING HOST MUST SUPPLY TWO SERVICES THAT ARE NOT PART OF THE DATAMODEL:
synchronous text measurement, and a frame subscription.** This section says what
they must DO. It does not say what they are called, and the difference is the
whole design of it.

## Why this is a section and not a class

Sprint 7 added both services to Dew and regenerated `datamodel_scope.md`. It came
back **byte-identical**: 136 of 138 properties, 37 of 52 members, unchanged.

That is the argument, and it was a measurement rather than a preference. Neither
service is a member of any class in the reflection database, so a document whose
generated half counts class members has nowhere to put them and would be wrong to
grow a hand-written list beside its generated one. This project has already paid
for a hand-maintained surface list once: it reported 35 of 138 for a surface the
host had never implemented, because the names in it were Aether's.

So the standard has two halves now. `datamodel_scope.md` is what a host must
ACCEPT -- classes, properties, members -- and is generated from the engine's own
reflection database. This is what a host must BE ABLE TO DO, and is written by
hand because there is nothing to generate it from.

## Why they are required at all

The short version: **a host that supplies neither runs no UI framework**, which
makes "conforming but unusable" a state the standard should not permit.

The long version is that both questions are unanswerable from inside the
DataModel, and this is a property of the questions rather than of any particular
host.

- **"How wide is this string?"** depends on the face the host will actually draw
  with, at the size it will actually draw at. No amount of instance tree contains
  it. A guest that guesses produces a layout that is wrong in a way that looks
  like a font bug.
- **"When does the next frame begin?"** depends on the host's own loop. A guest
  that polls burns a core; a guest that guesses animates against a clock nothing
  else is using.

Every implementation already has both. The engine has `TextService:GetTextSize` and
`RunService.Heartbeat`. Dew has them inside the host and, since sprint 7, reachable
from Luau. A Luau test double has to simulate both to be useful at all. The
standard is recording something all three already do, which is the right time to
record it.

## The two services

### 1. Text measurement

**A host MUST provide a way to ask, for a string and a size, what its natural
width and height would be, and MUST answer SYNCHRONOUSLY.**

    measure(text, size, font?) -> width, height

**Synchronous is the requirement, not a convenience.** A layout pass runs inside a
frame and returns a tree of resolved rectangles; it cannot await. A host that
answered asynchronously would not be slower, it would be unusable: every
auto-sized element would resolve to zero on the frame that needed it. This is why
the engine's `GetTextSize` is the shape it is, and why a host that has an
asynchronous measurement available must still expose a synchronous answer, even
an approximate one it later refines.

**NATURAL size, not wrapped size.** The question is how large the string wants to
be, not how it flows into a box. A caller that wants wrapping has a box already
and can ask about it.

**A host MAY be approximate and MUST NOT be inconsistent.** Two calls with the
same arguments must give the same answer within a frame; a measurement that
disagrees with the pixels the same host then draws is worse than no measurement,
because a layout built on it is wrong in a way that reads as a rasteriser bug.

### 2. A frame subscription

**A host MUST provide a way to be called once per frame with the elapsed time
since the previous frame, and the subscription MUST return a disposer.**

    onFrame(callback) -> unsubscribe

**The disposer is the requirement.** Without one, N subscribers means N host-level
subscriptions and no way to take any of them down; with one, a host implementation
can keep a single connection for N listeners and drop it when the last goes. That
is not a performance note -- it is what makes a framework able to subscribe from
inside a component scope at all, because a component that mounts and unmounts must
be able to leave nothing behind.

**A host MUST tolerate a listener that disposes itself, or a sibling, during the
callback.** This falls out of the previous paragraph: teardown happens in
response to a frame more often than at any other time. The usual implementation
is to snapshot the listener list per frame.

**A host MUST also provide a monotonic elapsed-time reading.**

    now() -> seconds

**Monotonic, and not a wall clock.** Its zero may be arbitrary; what matters is
that it never steps backwards, because animation subtracts two readings and a
daylight-saving change would make every spring in the process jump. A host that
also offers wall-clock time offers it separately, and that one is not this.

**A host that DRIVES its own frames MUST NOT offer a way to advance one by hand.**
A guest able to step the frame loop can run every frame twice. This is a
prohibition rather than an omission: a driven host may present the member and do
nothing, which is what the engine does, and MUST NOT present one that works.

## What is NOT required

**A name.** Nothing above says where these live or what they are called. The engine
reaches them through `game:GetService`, Dew puts them on its own `dew` global, a Luau
double returns them from a host table. All three conform. A standard that
specified the spelling would be specifying that every host be the engine, which is
what this whole standard exists not to do.

**Font selection, font loading, or a font registry.** The `font` argument above is
optional and a host with one face may ignore it. What a host must not do is accept
a font name and then measure a different face.

**Any particular accuracy.** See above: consistent with what the same host draws
is the requirement.

## How this reaches an implementation

A guest framework does not read this document; it declares two seams and lets each
host fill them. Aether's `Host` interface names them `Text` and `Clock`, which is
what made sprint 8's work small -- the seams already existed, and moving the engine's
two service lookups behind them changed nothing above Layer 2.5.

    engine   game:GetService("TextService"):GetTextSize   RunService.Heartbeat
    Dew      dew.Text.Measure                             dew.Clock.OnFrame
    double   a per-glyph advance table                    a simulated clock

`dew.Text` and `dew.Clock` are Dew's own spelling and this document does not
bless it. Sprint 7's record says so in its own words: the name is expected to
be revisited by whatever decision the standard makes, and it was chosen
precisely because it could not be mistaken for portable -- installing globals
called `TextService` and `RunService` would look like the engine and run
nowhere else.

## What this does not settle: where the standard lives

**FLAGGED, NOT RESOLVED.** This document is in `dew/docs/` because that is where
the generated half of the standard already is, and because a hand-written
companion to a generated document should be next to it. That is a reason to put it
here and not an argument that here is right.

The standard's documents are currently split across two repositories:

    dew/docs/datamodel_scope.md      generated; classes, properties, members
    dew/docs/host_services.md        this file
    aether/conformance/LAYOUT.md     layout behaviour
    aether/conformance/cases/*.luau  the cases, run by both implementations

**`LAYOUT.md` sitting in Aether is the odd one, and ADR-004 is why it now reads
oddly.** ADR-004 says Aether must "run on any host that adheres to the DataModel
Standard" and must not implement one; ADR-001 says layout must go native, and
roadmap step P makes layout a HOST job with a Rust runner over the same cases.
A document specifying behaviour that two hosts must implement, living inside the
repository of a consumer that implements neither, is a boundary this project has
already corrected once -- ADR-004 moved three Rust crates for the same reason.

Three shapes were considered and none is taken here:

| | |
| :--- | :--- |
| **The standard lives in Dew** | Simplest, and it is where two of the four documents already are. It also makes the standard look like one implementation's documentation of itself, which is exactly the confusion ADR-001 opens by describing |
| **The standard is its own repository** | Honest about what it is -- two implementations and at least one consumer, none of them owning it. Costs a fourth repository, a fourth CI, and a pin in each consumer, for four documents and a directory of cases |
| **Nothing moves; the split is documented** | This is the status quo plus a paragraph, and it is what this section is |

**Recommendation: not now, and not never.** The cases are the part that would
benefit most from a neutral home, and roadmap step P is when a second
implementation starts running them -- that is the moment the question stops being
tidiness and starts being about who owns a failing case. Deciding it before then
would commit to a layout for a standard whose second consumer does not exist yet.

Until it is decided, this file is the standard's host-services section wherever
the standard turns out to live, and moving it is a `git mv`.
