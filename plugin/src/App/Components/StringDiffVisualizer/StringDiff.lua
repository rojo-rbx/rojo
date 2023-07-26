--[[
    Based on DiffMatchPatch by Neil Fraser.
    https://github.com/google/diff-match-patch
]]

export type DiffAction = number
export type Diff = { actionType: DiffAction, value: string }
export type Diffs = { Diff }

local StringDiff = {
	ActionTypes = table.freeze({
		Equal = 0,
		Delete = 1,
		Insert = 2,
	}),
}

function StringDiff.findDiffs(text1: string, text2: string): Diffs
	-- Validate inputs
	if type(text1) ~= "string" or type(text2) ~= "string" then
		error(
			string.format(
				"Invalid inputs to StringDiff.findDiffs, expected strings and got (%s, %s)",
				type(text1),
				type(text2)
			),
			2
		)
	end

	-- Shortcut if the texts are identical
	if text1 == text2 then
		return { { actionType = StringDiff.ActionTypes.Equal, value = text1 } }
	end

	-- Trim off any shared prefix and suffix
	-- These are easy to detect and can be dealt with quickly without needing a complex diff
	-- and later we simply add them as Equal to the start and end of the diff
	local sharedPrefix, sharedSuffix
	local prefixLength = StringDiff._sharedPrefix(text1, text2)
	if prefixLength > 0 then
		-- Store the prefix
		sharedPrefix = string.sub(text1, 1, prefixLength)
		-- Now trim it off
		text1 = string.sub(text1, prefixLength + 1)
		text2 = string.sub(text2, prefixLength + 1)
	end

	local suffixLength = StringDiff._sharedSuffix(text1, text2)
	if suffixLength > 0 then
		-- Store the suffix
		sharedSuffix = string.sub(text1, -suffixLength)
		-- Now trim it off
		text1 = string.sub(text1, 1, -suffixLength - 1)
		text2 = string.sub(text2, 1, -suffixLength - 1)
	end

	-- Compute the diff on the middle block where the changes lie
	local diffs = StringDiff._computeDiff(text1, text2)

	-- Restore the prefix and suffix
	if sharedPrefix then
		table.insert(diffs, 1, { actionType = StringDiff.ActionTypes.Equal, value = sharedPrefix })
	end
	if sharedSuffix then
		table.insert(diffs, { actionType = StringDiff.ActionTypes.Equal, value = sharedSuffix })
	end

	-- Cleanup the diff
	diffs = StringDiff._reorderAndMerge(diffs)

	return diffs
end

function StringDiff._sharedPrefix(text1: string, text2: string): number
	-- Uses a binary search to find the largest common prefix between the two strings
	-- Performance analysis: http://neil.fraser.name/news/2007/10/09/

	-- Shortcut common cases
	if (#text1 == 0) or (#text2 == 0) or (string.byte(text1, 1) ~= string.byte(text2, 1)) then
		return 0
	end

	local pointerMin = 1
	local pointerMax = math.min(#text1, #text2)
	local pointerMid = pointerMax
	local pointerStart = 1
	while pointerMin < pointerMid do
		if string.sub(text1, pointerStart, pointerMid) == string.sub(text2, pointerStart, pointerMid) then
			pointerMin = pointerMid
			pointerStart = pointerMin
		else
			pointerMax = pointerMid
		end
		pointerMid = math.floor(pointerMin + (pointerMax - pointerMin) / 2)
	end

	return pointerMid
end

function StringDiff._sharedSuffix(text1: string, text2: string): number
	-- Uses a binary search to find the largest common suffix between the two strings
	-- Performance analysis: http://neil.fraser.name/news/2007/10/09/

	-- Shortcut common cases
	if (#text1 == 0) or (#text2 == 0) or (string.byte(text1, -1) ~= string.byte(text2, -1)) then
		return 0
	end

	local pointerMin = 1
	local pointerMax = math.min(#text1, #text2)
	local pointerMid = pointerMax
	local pointerEnd = 1
	while pointerMin < pointerMid do
		if string.sub(text1, -pointerMid, -pointerEnd) == string.sub(text2, -pointerMid, -pointerEnd) then
			pointerMin = pointerMid
			pointerEnd = pointerMin
		else
			pointerMax = pointerMid
		end
		pointerMid = math.floor(pointerMin + (pointerMax - pointerMin) / 2)
	end

	return pointerMid
end

function StringDiff._computeDiff(text1: string, text2: string): Diffs
	-- Assumes that the prefix and suffix have already been trimmed off
	-- and shortcut returns have been made so these texts must be different

	local text1Length, text2Length = #text1, #text2

	if text1Length == 0 then
		-- It's simply inserting all of text2 into text1
		return { { actionType = StringDiff.ActionTypes.Insert, value = text2 } }
	end

	if text2Length == 0 then
		-- It's simply deleting all of text1
		return { { actionType = StringDiff.ActionTypes.Delete, value = text1 } }
	end

	local longText = if text1Length > text2Length then text1 else text2
	local shortText = if text1Length > text2Length then text2 else text1
	local shortTextLength = #shortText

	-- Shortcut if the shorter string exists entirely inside the longer one
	local indexOf = if shortTextLength == 0 then nil else string.find(longText, shortText, 1, true)
	if indexOf ~= nil then
		local diffs = {
			{ actionType = StringDiff.ActionTypes.Insert, value = string.sub(longText, 1, indexOf - 1) },
			{ actionType = StringDiff.ActionTypes.Equal, value = shortText },
			{ actionType = StringDiff.ActionTypes.Insert, value = string.sub(longText, indexOf + shortTextLength) },
		}
		-- Swap insertions for deletions if diff is reversed
		if text1Length > text2Length then
			diffs[1].actionType, diffs[3].actionType = StringDiff.ActionTypes.Delete, StringDiff.ActionTypes.Delete
		end
		return diffs
	end

	if shortTextLength == 1 then
		-- Single character string
		-- After the previous shortcut, the character can't be an equality
		return {
			{ actionType = StringDiff.ActionTypes.Delete, value = text1 },
			{ actionType = StringDiff.ActionTypes.Insert, value = text2 },
		}
	end

	return StringDiff._bisect(text1, text2)
end

function StringDiff._bisect(text1: string, text2: string): Diffs
	-- Find the 'middle snake' of a diff, split the problem in two
	-- and return the recursively constructed diff
	-- See Myers 1986 paper: An O(ND) Difference Algorithm and Its Variations

	-- Cache the text lengths to prevent multiple calls
	local text1Length = #text1
	local text2Length = #text2

	local _sub, _element
	local maxD = math.ceil((text1Length + text2Length) / 2)
	local vOffset = maxD
	local vLength = 2 * maxD
	local v1 = table.create(vLength)
	local v2 = table.create(vLength)

	-- Setting all elements to -1 is faster in Lua than mixing integers and nil
	for x = 0, vLength - 1 do
		v1[x] = -1
		v2[x] = -1
	end
	v1[vOffset + 1] = 0
	v2[vOffset + 1] = 0
	local delta = text1Length - text2Length

	-- If the total number of characters is odd, then
	-- the front path will collide with the reverse path
	local front = (delta % 2 ~= 0)

	-- Offsets for start and end of k loop
	-- Prevents mapping of space beyond the grid
	local k1Start = 0
	local k1End = 0
	local k2Start = 0
	local k2End = 0
	for d = 0, maxD - 1 do
		-- Walk the front path one step
		for k1 = -d + k1Start, d - k1End, 2 do
			local k1_offset = vOffset + k1
			local x1
			if (k1 == -d) or ((k1 ~= d) and (v1[k1_offset - 1] < v1[k1_offset + 1])) then
				x1 = v1[k1_offset + 1]
			else
				x1 = v1[k1_offset - 1] + 1
			end
			local y1 = x1 - k1
			while
				(x1 <= text1Length)
				and (y1 <= text2Length)
				and (string.sub(text1, x1, x1) == string.sub(text2, y1, y1))
			do
				x1 = x1 + 1
				y1 = y1 + 1
			end
			v1[k1_offset] = x1
			if x1 > text1Length + 1 then
				-- Ran off the right of the graph
				k1End = k1End + 2
			elseif y1 > text2Length + 1 then
				-- Ran off the bottom of the graph
				k1Start = k1Start + 2
			elseif front then
				local k2_offset = vOffset + delta - k1
				if k2_offset >= 0 and k2_offset < vLength and v2[k2_offset] ~= -1 then
					-- Mirror x2 onto top-left coordinate system
					local x2 = text1Length - v2[k2_offset] + 1
					if x1 > x2 then
						-- Overlap detected
						return StringDiff._bisectSplit(text1, text2, x1, y1)
					end
				end
			end
		end

		-- Walk the reverse path one step
		for k2 = -d + k2Start, d - k2End, 2 do
			local k2_offset = vOffset + k2
			local x2
			if (k2 == -d) or ((k2 ~= d) and (v2[k2_offset - 1] < v2[k2_offset + 1])) then
				x2 = v2[k2_offset + 1]
			else
				x2 = v2[k2_offset - 1] + 1
			end
			local y2 = x2 - k2
			while
				(x2 <= text1Length)
				and (y2 <= text2Length)
				and (string.sub(text1, -x2, -x2) == string.sub(text2, -y2, -y2))
			do
				x2 = x2 + 1
				y2 = y2 + 1
			end
			v2[k2_offset] = x2
			if x2 > text1Length + 1 then
				-- Ran off the left of the graph
				k2End = k2End + 2
			elseif y2 > text2Length + 1 then
				-- Ran off the top of the graph
				k2Start = k2Start + 2
			elseif not front then
				local k1_offset = vOffset + delta - k2
				if k1_offset >= 0 and k1_offset < vLength and v1[k1_offset] ~= -1 then
					local x1 = v1[k1_offset]
					local y1 = vOffset + x1 - k1_offset
					-- Mirror x2 onto top-left coordinate system
					x2 = text1Length - x2 + 1
					if x1 > x2 then
						-- Overlap detected
						return StringDiff._bisectSplit(text1, text2, x1, y1)
					end
				end
			end
		end
	end

	-- Number of diffs equals number of characters, no commonality at all
	return {
		{ actionType = StringDiff.ActionTypes.Delete, value = text1 },
		{ actionType = StringDiff.ActionTypes.Insert, value = text2 },
	}
end

function StringDiff._bisectSplit(text1: string, text2: string, x: number, y: number): Diffs
	-- Given the location of the 'middle snake',
	-- split the diff in two parts and recurse

	local text1a = string.sub(text1, 1, x - 1)
	local text2a = string.sub(text2, 1, y - 1)
	local text1b = string.sub(text1, x)
	local text2b = string.sub(text2, y)

	-- Compute both diffs serially
	local diffs = StringDiff.findDiffs(text1a, text2a)
	local diffsB = StringDiff.findDiffs(text1b, text2b)

	-- Merge diffs
	table.move(diffsB, 1, #diffsB, #diffs + 1, diffs)
	return diffs
end

function StringDiff._reorderAndMerge(diffs: Diffs): Diffs
	-- Reorder and merge like edit sections and merge equalities
	-- Any edit section can move as long as it doesn't cross an equality

	-- Add a dummy entry at the end
	table.insert(diffs, { actionType = StringDiff.ActionTypes.Equal, value = "" })

	local pointer = 1
	local countDelete, countInsert = 0, 0
	local textDelete, textInsert = "", ""
	local commonLength
	while diffs[pointer] do
		local actionType = diffs[pointer].actionType
		if actionType == StringDiff.ActionTypes.Insert then
			countInsert = countInsert + 1
			textInsert = textInsert .. diffs[pointer].value
			pointer = pointer + 1
		elseif actionType == StringDiff.ActionTypes.Delete then
			countDelete = countDelete + 1
			textDelete = textDelete .. diffs[pointer].value
			pointer = pointer + 1
		elseif actionType == StringDiff.ActionTypes.Equal then
			-- Upon reaching an equality, check for prior redundancies
			if countDelete + countInsert > 1 then
				if (countDelete > 0) and (countInsert > 0) then
					-- Factor out any common prefixies
					commonLength = StringDiff._sharedPrefix(textInsert, textDelete)
					if commonLength > 0 then
						local back_pointer = pointer - countDelete - countInsert
						if
							(back_pointer > 1) and (diffs[back_pointer - 1].actionType == StringDiff.ActionTypes.Equal)
						then
							diffs[back_pointer - 1].value = diffs[back_pointer - 1].value
								.. string.sub(textInsert, 1, commonLength)
						else
							table.insert(diffs, 1, {
								actionType = StringDiff.ActionTypes.Equal,
								value = string.sub(textInsert, 1, commonLength),
							})
							pointer = pointer + 1
						end
						textInsert = string.sub(textInsert, commonLength + 1)
						textDelete = string.sub(textDelete, commonLength + 1)
					end
					-- Factor out any common suffixies
					commonLength = StringDiff._sharedSuffix(textInsert, textDelete)
					if commonLength ~= 0 then
						diffs[pointer].value = string.sub(textInsert, -commonLength) .. diffs[pointer].value
						textInsert = string.sub(textInsert, 1, -commonLength - 1)
						textDelete = string.sub(textDelete, 1, -commonLength - 1)
					end
				end
				-- Delete the offending records and add the merged ones
				pointer = pointer - countDelete - countInsert
				for _ = 1, countDelete + countInsert do
					table.remove(diffs, pointer)
				end
				if #textDelete > 0 then
					table.insert(diffs, pointer, { actionType = StringDiff.ActionTypes.Delete, value = textDelete })
					pointer = pointer + 1
				end
				if #textInsert > 0 then
					table.insert(diffs, pointer, { actionType = StringDiff.ActionTypes.Insert, value = textInsert })
					pointer = pointer + 1
				end
				pointer = pointer + 1
			elseif (pointer > 1) and (diffs[pointer - 1].actionType == StringDiff.ActionTypes.Equal) then
				-- Merge this equality with the previous one
				diffs[pointer - 1].value = diffs[pointer - 1].value .. diffs[pointer].value
				table.remove(diffs, pointer)
			else
				pointer = pointer + 1
			end
			countInsert, countDelete = 0, 0
			textDelete, textInsert = "", ""
		end
	end
	if diffs[#diffs].value == "" then
		-- Remove the dummy entry at the end
		diffs[#diffs] = nil
	end

	-- Second pass: look for single edits surrounded on both sides by equalities
	-- which can be shifted sideways to eliminate an equality
	-- e.g: A<ins>BA</ins>C -> <ins>AB</ins>AC
	local changes = false
	pointer = 2
	-- Intentionally ignore the first and last element (don't need checking)
	while pointer < #diffs do
		local prevDiff, nextDiff = diffs[pointer - 1], diffs[pointer + 1]
		if
			(prevDiff.actionType == StringDiff.ActionTypes.Equal)
			and (nextDiff.actionType == StringDiff.ActionTypes.Equal)
		then
			-- This is a single edit surrounded by equalities
			local currentDiff = diffs[pointer]
			local currentText = currentDiff.value
			local prevText = prevDiff.value
			local nextText = nextDiff.value
			if #prevText == 0 then
				table.remove(diffs, pointer - 1)
				changes = true
			elseif string.sub(currentText, -#prevText) == prevText then
				-- Shift the edit over the previous equality
				currentDiff.value = prevText .. string.sub(currentText, 1, -#prevText - 1)
				nextDiff.value = prevText .. nextDiff.value
				table.remove(diffs, pointer - 1)
				changes = true
			elseif string.sub(currentText, 1, #nextText) == nextText then
				-- Shift the edit over the next equality
				prevDiff.value = prevText .. nextText
				currentDiff.value = string.sub(currentText, #nextText + 1) .. nextText
				table.remove(diffs, pointer + 1)
				changes = true
			end
		end
		pointer = pointer + 1
	end

	-- If shifts were made, the diffs need reordering and another shift sweep
	if changes then
		return StringDiff._reorderAndMerge(diffs)
	end

	return diffs
end

return StringDiff
