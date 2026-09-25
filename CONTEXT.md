# Wherewolf

A Werewolf (social deduction) game for friends playing remotely from their phones' browsers, over an in-app voice and video call, with the game run by the machine instead of a human moderator.

## People

**Player**:
A person taking part in a Game. Every participant is a Player; nobody sits out to run the Game.
_Avoid_: User, participant

**Host**:
The Player who created the Lobby. Only they can configure the Roles and start the Game; once it starts they are an ordinary Player.
_Avoid_: Admin, owner, game master

**Narrator**:
The automated moderator that runs the Game: deals Roles, calls each Phase, collects actions and announces outcomes.
_Avoid_: Game master, GM, MJ, bot

**Spectator**:
A dead Player. They keep watching the whole Game, including every Night Turn and every Role, and can talk with other Spectators, but the living can no longer see or hear them.
_Avoid_: Ghost, observer, dead player

**Mayor**:
A title (not a Role) that the living Players elect on the first Day. The Mayor settles ties in the Day vote.
_Avoid_: Captain, sheriff, chief

## Game structure

**Lobby**:
The gathering place before a Game, reached by a shared link or code, where Players join with a display name.
_Avoid_: Room, waiting room

**Game**:
One complete session of Werewolf, from the moment the Host starts it until one Camp wins. Takes 5 to 12 Players.
_Avoid_: Match, party

**Phase**:
Either the Night or the Day. A Game alternates Night and Day until a Camp wins.
_Avoid_: Round, stage

**Night**:
The Phase in which the village sleeps and Roles act secretly, one Turn at a time.

**Day**:
The Phase in which all living Players see and hear each other, debate, and vote to eliminate someone.

**Vote**:
The Day decision in which living Players each designate someone to eliminate. The most-designated Player is eliminated; the Mayor breaks ties.
_Avoid_: Lynch, trial

**Victim**:
The Player the Werewolves choose to kill during their Turn — any living Player, a fellow Werewolf included. Dies at dawn unless the Witch saves them.
_Avoid_: Target, prey

**Elimination**:
Any Player's death, whether by the Werewolves, the Witch's poison, the Vote or the Hunter's shot. The Player becomes a Spectator, and their Role is revealed to everyone unless Hidden Roles is on.
_Avoid_: Kill (for the generic case), removal

**Election**:
The vote on the first Day in which the living Players choose the Mayor.
_Avoid_: Mayor vote (ambiguous with the Vote)

**Settings**:
The options the Host chooses in the Lobby before starting a Game: the Roles in play, the timers, and Hidden Roles.
_Avoid_: Config, options, rules

**Hidden Roles**:
A Setting in which an eliminated Player's Role is not revealed to the living. To avoid leaking who is dead, every Night Turn then lasts its full time even when its Role is dead. With Hidden Roles off, the Turns of dead Roles are skipped.
_Avoid_: Secret mode, blind mode

**Turn**:
The moment during the Night when one Role (or the Werewolves together) wakes up to act, while everyone else stays asleep.
_Avoid_: Step, action phase

**Moment**:
One stretch of a Game that the Narrator announces and times on its own: a Night Turn, the dawn announcement, the Day discussion, the Vote, its result, or the victory screen. The current Moment decides what each Player sees and may do.
_Avoid_: Step, stage

## Roles

**Role**:
The secret card dealt to a Player at the start of a Game, which decides their Camp and their abilities.
_Avoid_: Character, class, card

**Camp**:
The side a Player wins or loses with: the Village or the Werewolves.
_Avoid_: Team, faction

**Werewolf**, **Villager**, **Seer**, **Witch**, **Hunter**:
The Roles available in v1. All but the Werewolf belong to the Village.
