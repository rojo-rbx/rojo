--!strict
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
	diffs = StringDiff._cleanupSemantic(diffs)
	diffs = StringDiff._reorderAndMerge(diffs)

	return diffs
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

function StringDiff._cleanupSemantic(diffs: Diffs): Diffs
	-- Reduce the number of edits by eliminating semantically trivial equalities.
	local changes = false
	local equalities = {} -- Stack of indices where equalities are found.
	local equalitiesLength = 0 -- Keeping our own length var is faster.
	local lastEquality: string? = nil
	-- Always equal to diffs[equalities[equalitiesLength]].value
	local pointer = 1 -- Index of current position.
	-- Number of characters that changed prior to the equality.
	local length_insertions1 = 0
	local length_deletions1 = 0
	-- Number of characters that changed after the equality.
	local length_insertions2 = 0
	local length_deletions2 = 0

	while diffs[pointer] do
		if diffs[pointer].actionType == StringDiff.ActionTypes.Equal then -- Equality found.
			equalitiesLength = equalitiesLength + 1
			equalities[equalitiesLength] = pointer
			length_insertions1 = length_insertions2
			length_deletions1 = length_deletions2
			length_insertions2 = 0
			length_deletions2 = 0
			lastEquality = diffs[pointer].value
		else -- An insertion or deletion.
			if diffs[pointer].actionType == StringDiff.ActionTypes.Insert then
				length_insertions2 = length_insertions2 + #diffs[pointer].value
			else
				length_deletions2 = length_deletions2 + #diffs[pointer].value
			end
			-- Eliminate an equality that is smaller or equal to the edits on both
			-- sides of it.
			if
				lastEquality
				and (#lastEquality <= math.max(length_insertions1, length_deletions1))
				and (#lastEquality <= math.max(length_insertions2, length_deletions2))
			then
				-- Duplicate record.
				table.insert(
					diffs,
					equalities[equalitiesLength],
					{ actionType = StringDiff.ActionTypes.Delete, value = lastEquality }
				)
				-- Change second copy to insert.
				diffs[equalities[equalitiesLength] + 1].actionType = StringDiff.ActionTypes.Insert
				-- Throw away the equality we just deleted.
				equalitiesLength = equalitiesLength - 1
				-- Throw away the previous equality (it needs to be reevaluated).
				equalitiesLength = equalitiesLength - 1
				pointer = (equalitiesLength > 0) and equalities[equalitiesLength] or 0
				length_insertions1, length_deletions1 = 0, 0 -- Reset the counters.
				length_insertions2, length_deletions2 = 0, 0
				lastEquality = nil
				changes = true
			end
		end
		pointer = pointer + 1
	end

	-- Normalize the diff.
	if changes then
		StringDiff._reorderAndMerge(diffs)
	end
	StringDiff._cleanupSemanticLossless(diffs)

	-- Find any overlaps between deletions and insertions.
	-- e.g: <del>abcxxx</del><ins>xxxdef</ins>
	--   -> <del>abc</del>xxx<ins>def</ins>
	-- e.g: <del>xxxabc</del><ins>defxxx</ins>
	--   -> <ins>def</ins>xxx<del>abc</del>
	-- Only extract an overlap if it is as big as the edit ahead or behind it.
	pointer = 2
	while diffs[pointer] do
		if
			diffs[pointer - 1].actionType == StringDiff.ActionTypes.Delete
			and diffs[pointer].actionType == StringDiff.ActionTypes.Insert
		then
			local deletion = diffs[pointer - 1].value
			local insertion = diffs[pointer].value
			local overlap_length1 = StringDiff._commonOverlap(deletion, insertion)
			local overlap_length2 = StringDiff._commonOverlap(insertion, deletion)
			if overlap_length1 >= overlap_length2 then
				if overlap_length1 >= #deletion / 2 or overlap_length1 >= #insertion / 2 then
					-- Overlap found.  Insert an equality and trim the surrounding edits.
					table.insert(
						diffs,
						pointer,
						{ actionType = StringDiff.ActionTypes.Equal, value = string.sub(insertion, 1, overlap_length1) }
					)
					diffs[pointer - 1].value = string.sub(deletion, 1, #deletion - overlap_length1)
					diffs[pointer + 1].value = string.sub(insertion, overlap_length1 + 1)
					pointer = pointer + 1
				end
			else
				if overlap_length2 >= #deletion / 2 or overlap_length2 >= #insertion / 2 then
					-- Reverse overlap found.
					-- Insert an equality and swap and trim the surrounding edits.
					table.insert(
						diffs,
						pointer,
						{ actionType = StringDiff.ActionTypes.Equal, value = string.sub(deletion, 1, overlap_length2) }
					)
					diffs[pointer - 1] = {
						actionType = StringDiff.ActionTypes.Insert,
						value = string.sub(insertion, 1, #insertion - overlap_length2),
					}
					diffs[pointer + 1] = {
						actionType = StringDiff.ActionTypes.Delete,
						value = string.sub(deletion, overlap_length2 + 1),
					}
					pointer = pointer + 1
				end
			end
			pointer = pointer + 1
		end
		pointer = pointer + 1
	end

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

function StringDiff._commonOverlap(text1: string, text2: string): number
	-- Determine if the suffix of one string is the prefix of another.

	-- Cache the text lengths to prevent multiple calls.
	local text1_length = #text1
	local text2_length = #text2
	-- Eliminate the null case.
	if text1_length == 0 or text2_length == 0 then
		return 0
	end
	-- Truncate the longer string.
	if text1_length > text2_length then
		text1 = string.sub(text1, text1_length - text2_length + 1)
	elseif text1_length < text2_length then
		text2 = string.sub(text2, 1, text1_length)
	end
	local text_length = math.min(text1_length, text2_length)
	-- Quick check for the worst case.
	if text1 == text2 then
		return text_length
	end

	-- Start by looking for a single character match
	-- and increase length until no match is found.
	-- Performance analysis: https://neil.fraser.name/news/2010/11/04/
	local best = 0
	local length = 1
	while true do
		local pattern = string.sub(text1, text_length - length + 1)
		local found = string.find(text2, pattern, 1, true)
		if found == nil then
			return best
		end
		length = length + found - 1
		if found == 1 or string.sub(text1, text_length - length + 1) == string.sub(text2, 1, length) then
			best = length
			length = length + 1
		end
	end
end

function StringDiff._cleanupSemanticScore(one: string, two: string): number
	-- Given two strings, compute a score representing whether the internal
	-- boundary falls on logical boundaries.
	-- Scores range from 6 (best) to 0 (worst).

	if (#one == 0) or (#two == 0) then
		-- Edges are the best.
		return 6
	end

	-- Each port of this function behaves slightly differently due to
	-- subtle differences in each language's definition of things like
	-- 'whitespace'.  Since this function's purpose is largely cosmetic,
	-- the choice has been made to use each language's native features
	-- rather than force total conformity.
	local char1 = string.sub(one, -1)
	local char2 = string.sub(two, 1, 1)
	local nonAlphaNumeric1 = string.match(char1, "%W")
	local nonAlphaNumeric2 = string.match(char2, "%W")
	local whitespace1 = nonAlphaNumeric1 and string.match(char1, "%s")
	local whitespace2 = nonAlphaNumeric2 and string.match(char2, "%s")
	local lineBreak1 = whitespace1 and string.match(char1, "%c")
	local lineBreak2 = whitespace2 and string.match(char2, "%c")
	local blankLine1 = lineBreak1 and string.match(one, "\n\r?\n$")
	local blankLine2 = lineBreak2 and string.match(two, "^\r?\n\r?\n")

	if blankLine1 or blankLine2 then
		-- Five points for blank lines.
		return 5
	elseif lineBreak1 or lineBreak2 then
		-- Four points for line breaks
		-- DEVIATION: Prefer to start on a line break instead of end on it
		return if lineBreak1 then 4 else 4.5
	elseif nonAlphaNumeric1 and not whitespace1 and whitespace2 then
		-- Three points for end of sentences.
		return 3
	elseif whitespace1 or whitespace2 then
		-- Two points for whitespace.
		return 2
	elseif nonAlphaNumeric1 or nonAlphaNumeric2 then
		-- One point for non-alphanumeric.
		return 1
	end
	return 0
end

function StringDiff._cleanupSemanticLossless(diffs: Diffs)
	-- Look for single edits surrounded on both sides by equalities
	-- which can be shifted sideways to align the edit to a word boundary.
	-- e.g: The c<ins>at c</ins>ame. -> The <ins>cat </ins>came.

	local pointer = 2
	-- Intentionally ignore the first and last element (don't need checking).
	while diffs[pointer + 1] do
		local prevDiff, nextDiff = diffs[pointer - 1], diffs[pointer + 1]
		if
			(prevDiff.actionType == StringDiff.ActionTypes.Equal)
			and (nextDiff.actionType == StringDiff.ActionTypes.Equal)
		then
			-- This is a single edit surrounded by equalities.
			local diff = diffs[pointer]

			local equality1 = prevDiff.value
			local edit = diff.value
			local equality2 = nextDiff.value

			-- First, shift the edit as far left as possible.
			local commonOffset = StringDiff._sharedSuffix(equality1, edit)
			if commonOffset > 0 then
				local commonString = string.sub(edit, -commonOffset)
				equality1 = string.sub(equality1, 1, -commonOffset - 1)
				edit = commonString .. string.sub(edit, 1, -commonOffset - 1)
				equality2 = commonString .. equality2
			end

			-- Second, step character by character right, looking for the best fit.
			local bestEquality1 = equality1
			local bestEdit = edit
			local bestEquality2 = equality2
			local bestScore = StringDiff._cleanupSemanticScore(equality1, edit)
				+ StringDiff._cleanupSemanticScore(edit, equality2)

			while string.byte(edit, 1) == string.byte(equality2, 1) do
				equality1 = equality1 .. string.sub(edit, 1, 1)
				edit = string.sub(edit, 2) .. string.sub(equality2, 1, 1)
				equality2 = string.sub(equality2, 2)
				local score = StringDiff._cleanupSemanticScore(equality1, edit)
					+ StringDiff._cleanupSemanticScore(edit, equality2)
				-- The >= encourages trailing rather than leading whitespace on edits.
				if score >= bestScore then
					bestScore = score
					bestEquality1 = equality1
					bestEdit = edit
					bestEquality2 = equality2
				end
			end
			if prevDiff.value ~= bestEquality1 then
				-- We have an improvement, save it back to the diff.
				if #bestEquality1 > 0 then
					diffs[pointer - 1].value = bestEquality1
				else
					table.remove(diffs, pointer - 1)
					pointer = pointer - 1
				end
				diffs[pointer].value = bestEdit
				if #bestEquality2 > 0 then
					diffs[pointer + 1].value = bestEquality2
				else
					table.remove(diffs, pointer + 1)
					pointer = pointer - 1
				end
			end
		end
		pointer = pointer + 1
	end
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
