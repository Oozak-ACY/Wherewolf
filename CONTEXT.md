# Wherewolf

A Werewolf (social deduction) game for friends sitting around one table, each with their phone, with the game run by the machine instead of a human moderator: the Narrator's voice calls the Game out loud, and Players act secretly on their phones. Remote play (over an in-app voice and video call) is deferred to a later Mode; terms marked _Remote only_ belong to it.

## People

**Player**:
A person taking part in a Game. Every participant is a Player; nobody sits out to run the Game.
_Avoid_: User, participant

**Host**:
The Player who created the Lobby. Only they can configure the Roles and start the Game. Their phone is the Table device. Once the Game starts they play as an ordinary Player, except that until a Mayor is elected they may end the Day's debate early, even if they have been eliminated.
_Avoid_: Admin, owner, game master

**Narrator**:
The automated moderator that runs the Game: deals Roles, calls each Phase out loud from the Table device, collects actions from the Players' phones and announces outcomes.
_Avoid_: Game master, GM, MJ, bot

**Table device**:
The Host's phone, put on loudspeaker: the only device that gives the Narrator its voice for the whole table. Every other phone stays silent, and private information (a Role, the Seer's inspection) is only ever shown on its owner's screen, never spoken.
_Avoid_: Speaker, main screen, host phone

**Spectator**:
A dead Player, or someone who joined after the Game started (they play the next Game). Their phone shows the whole Game, including every Night Turn and every Role; at the table they stay silent. _Remote only:_ they can talk with other Spectators, but the living can no longer see or hear them.
_Avoid_: Ghost, observer, dead player

**Disconnected**:
A Player whose browser has lost contact. They keep their seat and reclaim it by reopening the link on the same browser. In the Lobby, a Player who stays Disconnected for the whole Grace period loses their seat as if they had Left; once the Game has started, their seat is always kept. The Host cannot start the Game while anyone is Disconnected. While the Host is Disconnected during a Game, the table has no Narrator voice, so the Game is Paused.
_Avoid_: Offline, AFK, absent

**Mayor**:
A title (not a Role) that the living Players elect on the first Day; a tied Election is drawn by lot. The Mayor may end the Day's debate early, and settles ties in the Vote (if they don't in time, nobody is eliminated). When eliminated, the Mayor names a living successor, or one is drawn by lot.
_Avoid_: Captain, sheriff, chief

## Game structure

**Lobby**:
The gathering place before a Game, reached by a shared link or code, where Players join with a display name.
_Avoid_: Room, waiting room

**Paused**:
The state of a Game while the Host is Disconnected: every timer stops and the Narrator stays silent until the Host reclaims their seat, then the Game resumes exactly where it stopped.
_Avoid_: Frozen, on hold

**Game**:
One complete session of Werewolf, from the moment the Host starts it until one Camp wins, or until nobody is left alive (a draw). Takes 5 to 12 Players. It opens with each Player seeing their Role on their phone; the first Night falls only once every living Player has confirmed they have seen it (the Role reveal).
_Avoid_: Match, party

**Phase**:
Either the Night or the Day. A Game alternates Night and Day until it ends.
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
Any Player's death, whether by the Werewolves, the Witch's poison, the Vote, the Hunter's shot or grief. The Player becomes a Spectator, and their Role is revealed to everyone unless Hidden Roles is on.
_Avoid_: Kill (for the generic case), removal

**Death trigger**:
What an Elimination sets off: the Hunter's shot, the Mayor naming a successor (a Hunter who is also Mayor shoots first), or a Lover dying of grief, which the Narrator announces as such. Triggers are resolved one at a time, in the order the deaths happened, right after the announcement of those deaths (the dawn, the Vote's result, or a previous shot); a death they cause can set off another. Victory is checked only once every trigger is resolved. Until their trigger is resolved, the eliminated Player stays at the table rather than becoming a Spectator: the living see and hear them, and they know only what the living know.
_Avoid_: On-death effect, last will

**Election**:
The vote on the first Day in which the living Players choose the Mayor.
_Avoid_: Mayor vote (ambiguous with the Vote)

**Settings**:
The options the Host chooses in the Lobby before starting a Game: the Roles in play, the timers, Hidden Roles and the Grace period.
_Avoid_: Config, options, rules

**Grace period**:
A Setting: how long a Disconnected Player keeps their seat in the Lobby before it is freed as if they had Left (60 seconds by default). Changing it applies at once to Players already Disconnected. It counts from the moment the Player disconnected, or from the return to the Lobby for a Player still Disconnected when a Game ends. A Player whose seat was freed comes back as a newcomer.
_Avoid_: Timeout, kick, AFK delay

**Feigned Turn**:
With Hidden Roles on, a Night Turn where nothing can happen: the Seer's or the Witch's when she is dead, or the Witch's when she has no potion she could use tonight. It lasts a random time, from a minimum set in the timers (10 seconds by default) up to the Turn's own timer, so that its length looks like a living Role taking their time.
_Avoid_: Fake turn, dummy turn

**Hidden Roles**:
A Setting in which an eliminated Player's Role is not revealed to the living. To avoid leaking who is dead, a dead Role's Night Turn then still takes place, as a Feigned Turn; a living Role's Turn ends as soon as its action is confirmed, as with Hidden Roles off. With Hidden Roles off, the Turns of dead Roles are skipped.
_Avoid_: Secret mode, blind mode

**Turn**:
The moment during the Night when one Role (or the Werewolves together) wakes up to act, while everyone else stays asleep. It ends as soon as its action is confirmed on the phones (every Werewolf confirming the same Victim, for the Werewolves), or when its timer runs out.
_Avoid_: Step, action phase

**Visibility plan**:
_Remote only._ Who may see and hear whom at the current Moment, as the Narrator decides it: the living see each other by Day, only the Werewolves see each other during their Turn, and Spectators see everyone while no living Player sees them. The media server enforces it.
_Avoid_: Permissions, rooms

**Moment**:
One stretch of a Game that the Narrator announces and times on its own: the Role reveal (untimed: it ends once every living Player has confirmed seeing their Role), a Night Turn, the dawn announcement, the Day discussion, the Election and its result, the Vote, the Mayor's tie-break, the Vote's result, the Hunter's shot, the Mayor's succession, the result of each, or the victory screen. The current Moment decides what each Player sees and may do.
Each Moment opens only once the Narrator has announced it: while the Table device says its lines, the Moment is held (no timer, nobody acts in it), every phone that isn't asleep shows the same neutral waiting screen, and the acting Role sleeps on until their Turn opens. It opens when the Table device has said its last line, or after 30 seconds if it never says so. A change the Narrator says nothing new about, like the Seer reading what she saw, is not announced.
_Avoid_: Step, stage

## Roles

**Role**:
The secret card dealt to a Player at the start of a Game, which decides their Camp and their abilities.
_Avoid_: Character, class, card

**Camp**:
The side a Player wins or loses with: the Village, the Werewolves, or the Lovers.
_Avoid_: Team, faction

**Werewolf**, **Villager**, **Seer**, **Witch**, **Hunter**, **Cupid**, **Thief**:
The Roles available. All but the Werewolf belong to the Village.

**Thief**:
A Village Role that acts only on the second Night, in the first Turn: the Thief may choose a living Player to rob, or nobody. The Theft takes effect at dawn: the Thief takes the robbed Player's Role as it stands (a spent potion stays spent), and the robbed Player holds a Villager card and plays with the Village from then on. The Mayor title and the Lovers bond stay with the robbed Player. A Thief who robs nobody plays as a Villager.

**Theft**:
The Thief's swap of Roles, taking effect at the dawn after the second Night, once that dawn's deaths and their Death triggers are resolved and before victory is checked. Until then the robbed Player keeps acting with their Role. It is cancelled if the Thief or the robbed Player died that Night. Whenever a Thief is in the Settings, the Narrator says at that dawn that a house was robbed and everyone should check their card, whether or not a Theft took place.
_Avoid_: Steal, swap

**Cupid**:
A Village Role that acts only on the first Night: Cupid names two different Players (themself allowed) as the Lovers. If their timer runs out, the Narrator draws the two at random.

**Lovers**:
The two Players Cupid named. Right after Cupid's Turn they wake together in their own Turn: each sees who their partner is (not their Role) and confirms. The bond is between the two Players, not their Roles. When one Lover is eliminated, the other dies of grief, as a Death trigger. Their Camp follows their current Roles (so a Theft can change it): two Lovers from the same Camp keep it; a Werewolf and a non-Werewolf form the Lovers Camp, which wins only if they are the last two alive. Each keeps their Role's abilities.
_Avoid_: Couple, lovebirds
