# Map-related design notes

## Map making

Stages are roughly:

- extract parcels inside a bbox from a .kml
- load elevation into memory from a .hgt
- get raw OSM ways from a .osm
- (elevation, raw OSM ways) -> split up OSM stuff
- find and remove disconnected things, then also compute bbox of result
- merge in the parcels fitting the specific bbox
- load traffic signal from a .shp and match to nearest intersection

- create finalish Intersection structs
- * split roads into lanes based on lane specs. also update Intersections.
- * trim road lines for each intersection
- * make turns for each intersection
- * make each building, finding the front path using lanes
- map over parcels directly

The live edits will modify lane specs and turns. Will have to re-do starred
items most likely. Should be straightforward to only redo small parts of those
stages.

## Lanes

It's time to model more things:

- multiple driving lanes, with possibly individual turn restrictions
- dedicated bus lanes
- lanes with parked cars
- bike lanes
- sidewalks

Each lane needs some geometry:

- center lines to draw agents on
	- for sidewalks, use center line to to draw agents on the left and right sides?
- polygons to draw the lane and mouseover

Open questions:

- Can we assume all lanes are the same width?
	- Seems wrong for many sidewalks especially
	- Could be wrong for bike lanes, but could just assume it's a bike lane with a buffer
- Some lanes are immutable
	- Sidewalks can't be changed to other types; they're raised with a curb

Some modeling questions:

- Where should expansion of roads into lanes happen?
	- initial OSM conversion, adding more stuff to the proto?
	- initial map_model::new loading, at least for development convenience
		- same reason that turns aren't (yet) serialized
- Is it useful to model the entire road?
	- the parent/child relation may be hard to maintain
	- but lanes need to know their siblings
	- maintaining directional sanity could be useful
	- what's the UI for changing lane types?
	- it's a bit arbitrary which lane should draw the yellow center lines



Initial design:
- "Road" becomes "Lane" with a type
- don't need to know sibling lanes yet
- arbitrarily, one lane might have extra bits/geometry for yellow center line markings
- ideally, get rid of one-wayness and original center points, and plumb along pre-shifted lines
	- but due to the polyline problem (affecting both geom center line layer that agents follow, and polygons for drawing), can't do this. encapsulate the messiness at least.
	- so, store one way and orig points and index, but have an accessor
	- as a compromise, dont interpet OSM points on a one-way road as the center, but as the edge? this is proving hard to do.

Thinking about a new design:
- Much more general "Land" primitive that's just a nice polygon boundary for drawing/selection and one (or more, for sidewalks?) center lines for how to cross the space, with a notion of turns. It's what road is now, but way simpler data.
- Maybe the GeomRoad / DrawRoad split is a little confusing after all, since the layering just isn't perfect. figure out the polygon and centerline up-front, then ditch the other intermediate gunk.
- also ideally make one polygon for the road, not a bunch of individual pieces? but then we'd have to go triangulate later for opengl anyway
- enforce that all the polygons are nonoverlapping

## Representing map edits

Two reasons for edits:
- the basemap is wrong because of bad OSM data or heuristics
- here's a possible edit to A/B test

Types of edits:
- change lane type between driving, parking, biking
	- sidewalks are fixed!
	- some edits are illegal... parking lane has to be in a certain side... right? well, actually, dont do that yet.
- delete a lane (because the basemap is wrong)
- modify stop sign priorities
- modify traffic signal timings

How to visually diff edits?
- highlight them
- UI to quickly jump and see them

How to encode the edits?
- "Remove lane" is weird; how about per road, list the lane types? Then it's
  almost kinda obvious how to plug into part of the current map making
pipeline.
- alright, let's really first think about road vs lane

Need to work through some edits to see how they affect downstream things. What
needs to be recomputed? How do we long-term serialize things like edits? How
can they even refer to things by ID if the IDs could change? What IDs might
change?

Alright, now we can be concrete -- when we have a road edit, what can be affected?

MAP LAYER:

- the road struct state (just list of children, really)
	- dont want to blindly run all the road making code, since it'd double-add stuff to intersection
- delete old lanes, make new lanes
	- how would IDs work? if we try to reuse the old ones, we might wind up
	  with gaps, or overflowing available space.
- trim lanes
	- need to recalculate original lane_center_pts for all affected lanes
	  in a certain direction. tricky since they're two-sided; have to
	  restore just the original direction on it.
- recalculate turns, for the two intersections
	- same ID problem
- recalculate some building front paths, maybe

CONTROL LAYER:

- recalculate two intersections

SIM LAYER:

- creating/deleting sidewalks is pretty easy
- SimQueues are associated with turns and lanes, but easyish to create/delete later
- should probably have a way to prevent mutations; maybe need to drain a lane of agents before changing it

UI:

- make a new DrawLane, DrawIntersection, etc
- update quadtrees
- would have to maybe update a bunch of plugin state (highlighting or
  floodfilling or something), but since we know road editor is active, is easy!



Strategies:
- testing via equivalence -- reload from scratch should be equal to live edits
	- will IDs make this very tricky?
- for things like sim and UI that hook on and have derived state, should we
  always kinda lazily grab DrawRoads, SimQueues, etc? or immediately plumb
  through deletes and inserts?
- is there a way to programatically record data dependencies or kinda do FRPish stuff from the start?
- could always blindly recalculate everything live, but man, that's gotta be slow
- maybe change constructors that take full map into incremental "hey, this road exists!" mutations. then just need to introduce deletions. in other words, embrace incremental mutability.
- assume the bbox doesn't change as a result of any edit



the ID problem:
- need determinism and deep equality checks for things. if we load a map from
  scratch with edits, vs do a live shuffle, the IDs wont match up if they use a
  slotmap.
- can we refer to things in more stable ways; no LaneID, but
  RoadID+direction+offset. no Turn, but two... effectively lane IDs?
- maybe we can combine these ideas; use nondet slotmaps, but when doing
  equality checks, dont use these IDs -- treat these IDs as memory addresses.
  IDs for lookup and IDs for equality.
- what're the different things that need this?
	- stable objects: building, intersection, parcel, road
	- malleable
		- lane (road, direction, offset, lane type)
		- turn (src lane, dst lane)
			- recurse and refer to full lane descriptions, or their temporary ID?
- ideally want to store things contiguously in memory
- ideally want a compact, easy thing to type quickly to debug.
- aka, ideally want a nice bijection from the persistent thing to numbers?
- actually, if we "leave room for" enough lanes per road and turns per intersection to begin with...
	- can just replace existing IDs when we change something
	- still have to mark things dead
	- still have to watch out for dangling references


The changes needed:
- figure out the ID problem
- change existing code from big constructors to incremental adds
	- exactly what layers and objects?
- implement incremental deletes
- try doing a live edit and comparing with from scratch


Going to start implementing part of this in a branch, just to get more detail.

- when there's a road edit, calculate the affected objects (road and all children, two intersections)
- implement a sanity check to make sure no dangling ref to old IDs

I think this is working so far. The vital question: is it too complicated? Is there a simpler way?
- simpler idea: retain more raw data, violently destroy road and intersection and make from scratch
	- problem: it'd percolate, we need to keep old connected roads the same
- EVEN SIMPLER IDEA: stop trying to solve hard problems
	- lane deletion is rare and a basemap-only edit; can mark it in the UI temporarily and omit in the next full load
	- changing lane types is the main intended edit. what actual consequences does this have? filtering through the list earlier...
		- change lane type
		- recalculate all turns for two intersections
			- the number of turns might go up or down
		- control layer intersection policies then need updating
		- sim needs to know about changed lanes and turns
		- and a few easy edits in the UI layer too
	- changing lane direction might be a little more complicated, but NOT BY MUCH

so, I think the steps:
= see what's useful from this branch, bring it to master (encapsulating the driving state stuff)
= ditch TurnIDs; have a BTreeMap of src/dst (LaneID, LaneID)
= add a mutate_lanes() and replace_turns() to all the appropriate layers

Cool, good enough to start. whew.

## Notes on King County GIS datasets

- TODO: https://data-seattlecitygis.opendata.arcgis.com/datasets/channelization

- https://data-seattlecitygis.opendata.arcgis.com/datasets/street-signs