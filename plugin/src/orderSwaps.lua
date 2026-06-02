--[[
	Determines the order in which `ServeSession:__replaceInstances` should swap
	instances so that sibling order is preserved.

	Roblox appends to `GetChildren()` on every reparent, so the order in which we
	re-parent replacements determines their final sibling order. To rebuild
	`GetChildren()` exactly as it was before the swap we must:

	* process ancestors before descendants, so each replacement's parent already
	  exists when we re-parent the replacement, and
	* process siblings in their original `GetChildren()` order.

	`swaps` is an array of `{ id, replacement, oldInstance }` entries. This sorts
	the array in place (annotating each entry with `depth`/`siblingIndex`) and
	returns it.
]]
local function orderSwaps(swaps)
	for _, swap in swaps do
		local depth = 0
		local ancestor = swap.oldInstance.Parent
		while ancestor ~= nil do
			depth += 1
			ancestor = ancestor.Parent
		end
		swap.depth = depth

		local siblingIndex = 0
		if swap.oldInstance.Parent ~= nil then
			siblingIndex = table.find(swap.oldInstance.Parent:GetChildren(), swap.oldInstance) or 0
		end
		swap.siblingIndex = siblingIndex
	end

	table.sort(swaps, function(a, b)
		if a.depth ~= b.depth then
			return a.depth < b.depth
		end
		return a.siblingIndex < b.siblingIndex
	end)

	return swaps
end

return orderSwaps
