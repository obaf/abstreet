# TODO - Refactoring

- easier way to define magic tuneable constants
	- and maybe to recalculate fixedish things if they change?

## Map layer

- fixed precision math
	- more careful geom types, with negative/positive cases
	- also bounds?
	- cant get rid of the ccw intersection check... different answer in some cases that looks bad

- maybe also the time to split into different lane types? what's similar/not between them?
	- graph querying?
	- rendering (and other UI/editor interactions)?
	- sim state?
	- Sidewalk, Parking, Street

- make synthetic use raw stuff directly?
	- lonlat vs pt is annoying; have to use bounds to balloon to world at least once

## Sim layer

- rename Car->Vehicle?

## ezgui layer

- probably use f32, not f64 everywhere... but after Pt2D becomes fixed size
- undo the y inversion hacks at last!
- ezgui passes EventCtx and DrawCtx with appropriate things exposed.
	- maybe move glyph ownership out of canvas entirely. dont need RefCell.
		- need to pass around a NonDrawCtx very uniformly first for this to work
	- canvas owning text-drawing is maybe a bit weird, at least API-wise
	- hide stuff inside the ctx's? canvas and prerender shouldnt even be known outside of crate
- generic World with quadtree should have actions on objects
- loading screen
	- cleanup hack: dont put glyphbrush in canvas at all
	- FileWithProgress should go directly into Timer
		- need to understand lifetimes
	- cleanup abstutil Timer stuff generally
