local RouteMap = require(script.Parent.RouteMap)

local function classEqual(a, b)
	if a == "*" or b == "*" then
		return true
	end

	return a == b
end

local function applyProperties(target, properties)
	for key, property in pairs(properties) do
		-- TODO: Transform property value based on property.Type
		-- Right now, we assume that 'value' is primitive!
		target[key] = property.Value
	end
end

--[[
	Attempt to parent `rbx` to `parent`, doing nothing if:
	* parent is already `parent`
	* Changing parent threw an error
]]
local function reparent(rbx, parent)
	if rbx then
		if rbx.Parent == parent then
			return
		end

		-- It's possible that 'rbx' is a service or some other object that we
		-- can't change the parent of. That's the only reason why Parent would
		-- fail except for rbx being previously destroyed!
		pcall(function()
			rbx.Parent = parent
		end)
	end
end

--[[
	Attempts to match up Roblox instances and object specifiers for
	reconciliation.

	An object is considered a match if they have the same Name and ClassName.

	primaryChildren and secondaryChildren can each be either a list of Roblox
	instances or object specifiers. Since they share a common shape, switching
	the two around isn't problematic!

	visited is expected to be an empty table initially. It will be filled with
	the set of children that have been visited so far.
]]
local function findNextChildPair(primaryChildren, secondaryChildren, visited)
	for _, primaryChild in ipairs(primaryChildren) do
		if not visited[primaryChild] then
			visited[primaryChild] = true

			for _, secondaryChild in ipairs(secondaryChildren) do
				if classEqual(primaryChild.ClassName, secondaryChild.ClassName) and primaryChild.Name == secondaryChild.Name then
					visited[secondaryChild] = true

					return primaryChild, secondaryChild
				end
			end

			return primaryChild, nil
		end
	end

	return nil, nil
end

local Reconciler = {}
Reconciler.__index = Reconciler

function Reconciler.new()
	local reconciler = {
		_routeMap = RouteMap.new(),
	}

	setmetatable(reconciler, Reconciler)

	return reconciler
end

--[[
	A semi-smart algorithm that attempts to apply the given item's children to
	an existing Roblox object.
]]
function Reconciler:_reconcileChildren(rbx, item)
	local visited = {}
	local rbxChildren = rbx:GetChildren()

	-- Reconcile any children that were added or updated
	while true do
		local itemChild, rbxChild = findNextChildPair(item.Children, rbxChildren, visited)

		if not itemChild then
			break
		end

		reparent(self:reconcile(rbxChild, itemChild), rbx)
	end

	-- Reconcile any children that were deleted
	while true do
		local rbxChild, itemChild = findNextChildPair(rbxChildren, item.Children, visited)

		if not rbxChild then
			break
		end

		reparent(self:reconcile(rbxChild, itemChild), rbx)
	end
end

--[[
	Construct a new Roblox object from the given item.
]]
function Reconciler:_reify(item)
	local className = item.ClassName

	-- "*" represents a match of any class. It reifies as a folder!
	if className == "*" then
		className = "Folder"
	end

	local rbx = Instance.new(className)
	rbx.Name = item.Name

	applyProperties(rbx, item.Properties)

	for _, child in ipairs(item.Children) do
		reparent(self:_reify(child), rbx)
	end

	if item.Route then
		self._routeMap:insert(item.Route, rbx)
	end

	return rbx
end

--[[
	Clears any state that the Reconciler has, stopping it completely.
]]
function Reconciler:destruct()
	self._routeMap:destruct()
end

--[[
	Apply the changes represented by the given item to a Roblox object that's a
	child of the given instance.
]]
function Reconciler:reconcile(rbx, item)
	-- Item was deleted
	if not item then
		if rbx then
			self._routeMap:removeByRbx(rbx)
			rbx:Destroy()
		end

		return nil
	end

	-- Item was created!
	if not rbx then
		return self:_reify(item)
	end

	-- Item changed type!
	if not classEqual(rbx.ClassName, item.ClassName) then
		self._routeMap:removeByRbx(rbx)
		rbx:Destroy()

		return self:_reify(item)
	end

	applyProperties(rbx, item.Properties)
	self:_reconcileChildren(rbx, item)

	return rbx
end

function Reconciler:reconcileRoute(route, item, itemRoute)
	local parent
	local rbx = game

	for i = 1, #route do
		local piece = route[i]

		local child = rbx:FindFirstChild(piece)

		-- We should get services instead of making folders here.
		if rbx == game and not child then
			local success
			success, child = pcall(game.GetService, game, piece)

			-- That isn't a valid service!
			if not success then
				child = nil
			end
		end

		-- We don't want to create a folder if we're reaching our target item!
		if child == nil and i ~= #route then
			child = Instance.new("Folder")
			child.Parent = rbx
			child.Name = piece
		end

		parent = rbx
		rbx = child
	end

	-- Let's check the route map!
	if rbx == nil then
		rbx = self._routeMap:get(itemRoute)
	end

	rbx = self:reconcile(rbx, item)

	reparent(rbx, parent)
end

return Reconciler
