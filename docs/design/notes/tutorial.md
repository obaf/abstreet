# Tutorial mode

## Synthetic maps

For tests and tutorial mode, I totally need the ability to create little
synthetic maps in a UI. Should be different than the main UI.

What are the 'abstract' objects to manipulate?

- Intersections... just points
	- Move these, have the roads move too
- Ability to connect two intersections with a straight line road
	- Edit lane type list in each direction
	- This lets border nodes be created
- Place rectangular buildings

This should basically use raw_data as primitives... or actually, no. GPS would
be weird to work with, and roads should be expressed as the two intersections,
so we don't have to update coordinates when we move intersections.

How to map lanes to stuff that make/lanes.rs will like? Might actually be easy,
actually.

Ideally, would render the abstract thing in one pane, and live-convert to the
full map and display it with the editor code in the other pane. But as the
halloween experiment shows -- that'd require a fair bit more refactoring first.

## Playing the levels

I would say individual tutorial levels could just be running a scenario, except...

- I don't want to hand-write scenario JSON. Code greatly preferred.
- I might want to pop up some dialogs at the beginning / middle / end to explain stuff
	- and these dialogs don't belong in the main editor crate
- Need to have listeners inspect sim state to detect when the player is correct/wrong
- All of the editor's controls don't make sense in tutorial mode (defining a new scenario)

I almost want to extract most stuff from the editor crate into a reusable
library, then use it from a new tutorial mode crate and the existing editor. But...

- the editor's current plugin architecture doesnt allow for specifying plugin
  interactions, which feels very relevant if these become generic
- some plugins know about primary/secondary state and plugins. do the concepts
  of per UI and per map have to be generic too, or is that just for editor?

- maybe 'modes' are collections of mutually compatible plugins?
- it's weird that plugins always exist as singletons, with explicit inactive states. maybe need to have new(input) -> option<plugin> and make event() indicate when the plugin should be destroyed. then a mode is an invocation of these, with its tiny little list of active plugins (boxed or maybe not)
- and a tutorial level is just one of these modes, maybe with some kind of setup() that clobbers the map and sim.
- think of modes as just event handlers! straight line code. the plugin trait is maybe not so useful. can share stuff like toggleable layerlike things by a SUBROUTINE

### Simple workflows

Think about the tutorial mode as a FSM. For now, no live editing while a sim is running.

- first make the user go through some dialogs
	- still displaying the map in the background, but no interaction yet
- then be in static explore mode
	- can do: canvas movement, debug objects, hide objects, toggle layers, 
	- can do things that are 'active plugin' and eat keyboard, but should pop to previous state
		- display log console, save map edits


a_b_tests.rs       diff_worlds.rs         layers.rs                scenarios.rs      steep.rs
chokepoints.rs     draw_neighborhoods.rs  logs.rs                  search.rs         stop_sign_editor.rs
classification.rs  floodfill.rs           map_edits.rs             show_activity.rs  time_travel.rs
color_picker.rs    follow.rs              mod.rs                   show_owner.rs     traffic_signal_editor.rs
debug_objects.rs   geom_validation.rs     neighborhood_summary.rs  show_route.rs     turn_cycler.rs
diff_all.rs        hider.rs               road_editor.rs           sim_controls.rs   warp.rs



Gah, bite this off in slow pieces. First find layer-like things... things that
draw/hide stuff. Tend to have very simple activate/deactive controls. No sim interaction.

Or maybe with the exclusive editors...
- in the short term, could make a plugin that just delegates to a smaller list
	- except some are per-map and some are not
		- wouldnt need that concept if we dont store plugins when
		  theyre inactive and have a notion of what can simultaneously
		  be active...
- figure out what other plugins are valid alongside the exclusive editors...
	- should activating an editor reset toggleable layers and hidden stuff? that's debug state...
	- when is moving via mouse still valid? color picker (right?), neighborhood, road/intersection editors
	- intersection and road editors... just debug. and actually, is that even true?
	- running a sim / doing time travel shouldnt be valid


debug stuff: toggleable layers, hider, geom validation, floodfill

alright maybe we actually do have lots of exclusive states...
- the exclusive editors
	- note that if we're running an A/B test, none of these are valid! cant edit stuff during a/b test... just run.
- explore
	- sim controls | time travel
	- bunch of nonblocking stuff... chokepoints, classification, debug, diff all, diff trip, floodfill, follow...
		- different keys to deactivate them? :P
	- some blocking stuff... search, warp. when these're around, run them first


- dir structure... all the exclusive stuff side-by-side, and then a shared/ for stuff that might apply during different states
- the exclusive editors: a_b_tests.rs     draw_neighborhoods.rs  map_edits.rs    scenarios.rs         traffic_signal_editor.rs
color_picker.rs  road_editor.rs  stop_sign_editor.rs




maybe as an initial step, can we get rid of plugins per map vs per UI and just have one or the other?

- every plugin per map?
	- toggleable layers then arent shared... fine
	- logs is per UI? whoa, that's actually maybe messy!
	- sim ctrl, diff world/trip does need to be independent though.
- every plugin per UI?
	- when we load a new map from edits, still have to replace the world.
	- would have to argue that no plugin that keeps per-map state can run during A/B test mode!
		- or rather, that we cant swap while any of those plugins hold state!
		- show owner, turn cycler, debug, follow (need to recalculate)
		- time travel (needs a/b support generally)
		- show route (totally valid to swap while this is going... grrr.)



maybe step 1...
- make a single 'Mode' for exclusive editors
	- skip it if a secondary sim is present (aka in A/B mode)
	- it lives per UI, because of above condition
	- for now, impl it as a hierarchial plugin itself that just delegates
	- keep plugin trait for each of em for convenience in edit mode, though.
	- each of the editors can stop having inactive state. have new() that returns option

and probably step 2...
- start smaller, a Debug mode... stuff that shouldnt really be relevant in tutorial mode, for example
	- chokepoints, classification, floodfill, geom validation, hider, toggleable layers, steep
	- arguably some of these could stack, but I don't care much yet... dont worry about ambient plugins yet

	- each of the editors can stop having inactive state. have new() that returns option
	- the permanent ones (hider and toggleable layers) shouldnt even implement Plugin; theyre custom weirdness
- make a single 'Mode' for normal exploration
	- the blocking ones: warp
	- the ambient ones: debug objects, follow, neighborhood summary, show activity, show owner, show route, turn cycler
		- still represent the inactive state? for now, sure
		- have to solve the problem of overlapping keys to quit
	- what is search? should it be ambient or not?
	- dont forget neighborhood summary


	- this has to be completely per UI or completely per map
	- let a bunch of plugins run non-exclusively there, as relevant
		- AmbientPlugin trait, maybe? or maybe just explicitly call on each field in order
	- and still have a single blocking plugin possible, like warp

	thursday pick-up:
	- overlapping keys to quit stuff...
	- cant edit mode when sim is actively running
		- where does sim ctrler belong?

and step 3...
- dismantle the plugin abstraction in UI and probably also the trait. do something different for modes.
- clean up event vs new_event
- use Escape to quit most plugins, since it'll only be callable normally from some modes
- make it more clear that keys cant overlap... in each mode, specify the trigger key it uses?
	- except some of them are more conditional and that makes overlap fine
- can we get rid of PluginsPerUI almost? since we'll likely get rid of plugins entirely... yeah?
	- view and debug mode can coexist!

### Overlapping keys

Is there a way to know that ambient plugins in ViewMode don't use the same keys?

- when we construct them in ViewMode, could pass in a hardcoded key and visually see they're not used too much.
- could get fancier and put all of the keys in a struct, move them out as we create plugins, thereby using the compiler to check. :D
- what if some keys are only usable in some contexts and that's OK?
- what if the plugin uses multiple keys? pass in both, at the expense of losing some readability at the creation site...

### One UI logic to rule em all

Almost done organizing plugins. For the last stretch, I think I need to solve a few related problems...

- Some modes can coexist or not. I want to write a single simple Plugin-like thing to do the delegation, outside of UI.
- this single thing will be different for tutorial mode, but easily pull in common collections of stuff like SimMode and ViewMode.
- this single thing will understand primary/secondary, the state that's per map, and the state that's independent
- what's left in UI? there's so much there right now...

## Listening to sim

Callbacks get so confusing. How about SimMode just exposes the most recent
events, and other things can reach in and query. They're responsible for not
double-counting.

## Overall loop / splash screen

I don't really want the top menu active at all during the splash screen.
Probably have to make each application own this state instead, which I
suspected from early on. :) But UserInput is very entangled with stuff,
probably hard to do right now.

splash screen: logo and author
(in the bg, a map in screensaver mode, just zoomed in some amount bouncing around randomlyish)
- sandbox
	- choose map
- mission
	- list of curated problems, with description/maybe a picture
- tutorial
- about
- quit

maybe even some more things to guide flow:
- a/b test some edits
- edit mode


as a pause screen, add a - resume option too


and rethinking game modes...
- explore
- simulate
- edit

common functionality:
- search
- warp
- legend



other modes...
- scratchpad: map editing and ad-hoc simulation
	- saving map edits
- define/edit neighborhoods, scenarios, missions
	- very different controls should be available!
	- plugins: manage neighborhoods, manage scenarios
- run an a/b test
	- setting up a/b test spec, checking results
	- special osd or bar for time, current sim
	- work on trip diffing


maybe top menu changes by the mode! some stuff could be common (debug plugins?)

Forget top menu, modal menu, OSD, right-click menus, all the current GUI things. Start over from each mode -- how should it work ideally?
- how do we indicate what major mode we're in and explain how to get out? top menu?

- tutorial mode
- challenge creation mode
	- manage neighborhoods and scenarios
	- how should the main functions be chosen? load/save neighborhood/scenario
- interactive sandbox mode
	- spawn traffic
		- maybe choose the spawn tool, then can select a building or intersection or road?
			- dont even allow selection of things that dont make sense
	- persistent (but maybe hideable?) controls for sim, OSD showing time and agent count and stuff
- map edit mode
	- this makes sense as a separate thing, to visualize the edits and make sure to save them
	- and to make it clear that there's no mixing with a running sim
	- but how fluidly should this be enterable from the sandbox mode?
	- replace with OSD with a little summary thing.. "sidewalk of 5th Ave"
- debug mode
	- stuff like tooltips, warp, search only belong here... until i make more generally usable navigation tools


persisting anything as modes change is hard to do with the borrow checker. ex: modal menus within the edit mode, soon the core components like map and drawmap. when we're processing the current state, we &mut, but then we want to take ownership of the pieces, which should sorta be safe because we're replacing the overall state. solved this for screensaver because it's an Option, and we can take() it -- replace with None.
- can we just take ownership and return back at the end?
