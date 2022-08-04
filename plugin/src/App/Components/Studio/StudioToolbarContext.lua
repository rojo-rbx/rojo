local Rojo = script:FindFirstAncestor("Rojo")
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local StudioToolbarContext = Roact.createContext(nil)

return StudioToolbarContext
