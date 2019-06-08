There are a number of existing plugins for Roblox that move code from the filesystem into Roblox.

Besides Rojo, you might consider:

* [rbxmk by Anaminus](https://github.com/anaminus/rbxmk)
* [Rofresh by Osyris](https://github.com/osyrisrblx/rofresh)
* [RbxRefresh by Osyris](https://github.com/osyrisrblx/RbxRefresh)
* [Studio Bridge by Vocksel](https://github.com/vocksel/studio-bridge)
* [Elixir by Vocksel](https://github.com/vocksel/elixir)
* [RbxSync by evaera](https://github.com/evaera/RbxSync)
* [CodeSync by MemoryPenguin](https://github.com/MemoryPenguin/CodeSync)
* [rbx-exteditor by MemoryPenguin](https://github.com/MemoryPenguin/rbx-exteditor)

So why did I build Rojo?

Each of these tools solves what is essentially the same problem from a few different angles. The goal of Rojo is to take all of the lessons and ideas learned from these projects and build a tool that can solve this problem for good.

Additionally:

* I think that this tool needs to be built in a compiled language without a runtime, for easy distribution and good performance.
* I think that the conventions promoted by other sync plugins (`.module.lua` for modules, as well a single sync point) are sub-optimal.
* I think that I have a good enough understanding of the problem to build something robust.
* I think that Rojo should be able to do more than just sync code.