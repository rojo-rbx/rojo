# Why Rojo?
There are a number of existing plugins for Roblox that move code from the filesystem into Roblox.

Besides Rojo, there is:

* [Studio Bridge](https://github.com/vocksel/studio-bridge) by [Vocksel](https://github.com/vocksel)
* [RbxRefresh](https://github.com/osyrisrblx/RbxRefresh) by [Osyris](https://github.com/osyrisrblx)
* [RbxSync](https://github.com/evaera/RbxSync) by [evaera](https://github.com/evaera)
* [CodeSync](https://github.com/MemoryPenguin/CodeSync) and [rbx-exteditor](https://github.com/MemoryPenguin/rbx-exteditor) by [MemoryPenguin](https://github.com/MemoryPenguin)
* [rbxmk](https://github.com/anaminus/rbxmk) by [Anaminus](https://github.com/anaminus)

So why did I build Rojo?

Each of these tools solves what is essentially the same problem from a few different angles. The goal of Rojo is to take all of the lessons and ideas learned from these projects and build a tool that can solve the problem for good.

Additionally:

* I think that this tool needs to be built in a compiled language without a runtime, for easy distribution and good performance.
* I think that the conventions promoted by other sync plugins (`.module.lua` for modules, as well a single sync point) are sub-optimal.
* I think that I have a good enough understanding of the problem to build something robust.
* I think that Rojo should be able to do more than just sync code.