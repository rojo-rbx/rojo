Rojo is designed to be adopted incrementally. How much of your project Rojo manages is up to you!

There are two primary categories of ways to use Rojo: *Fully Managed*, where everything is managed by Rojo, and *Partially Managed*, where Rojo only manages a slice of your project.

## Fully Managed
In a fully managed game project, Rojo controls every instance. A fully managed Rojo project can be built from scratch using `rojo build`.

Fully managed projects are most practical for libraries, plugins, and simple games.

Rojo's goal is to make it practical and easy for _every_ project to be fully managed, but we're not quite there yet!

### Pros
* Fully reproducible builds from scratch
* Everything checked into version control

### Cons
* Without two-way sync, models have to be saved manually
    * This can be done with the 'Save to File...' menu in Roblox Studio
    * This will be solved by Two-Way Sync ([issue #164](https://github.com/LPGhatguy/rojo/issues/164))
* Rojo can't manage everything yet
    * Refs are currently broken ([issue #142](https://github.com/LPGhatguy/rojo/issues/142))

## Partially Managed
In a partially managed project, Rojo only handles a slice of the game. This could be as small as a couple scripts, or as large as everything except `Workspace`!

The rest of the place's content can be versioned using Team Create or checked into source control.

Partially managed projects are most practical for complicated games, or games that are migrating to use Rojo.

### Pros
* Easier to adopt gradually
* Integrates with Team Create

### Cons
* Not everything is in version control, which makes merges tougher
* Rojo can't live-sync instances like Terrain, MeshPart, or CSG operations yet
    * Will be fixed with plugin escalation ([issue #169](https://github.com/LPGhatguy/rojo/issues/169))