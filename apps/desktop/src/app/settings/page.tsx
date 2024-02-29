"use client";

import { useContext, useMemo, useState } from "react";

import { ThemeContext } from "../layout";

export default function Home() {
	const { lightMode, setLightMode } = useContext(ThemeContext);

	const lightModeText = useMemo(() => (lightMode ? "Dark Mode" : "Light Mode"), [lightMode]);

	const [username, setUsername] = useState("<Username from Tauri>");
	const [bio, setBio] = useState("<Bio from Tauri>");

	return (
		<div className="flex justify-center">
			<div className="py-5 w-1/3 space-y-1">
				<div className="flex justify-center bg-[#113355] text-[white] px-4 py-2 rounded-md">
					<button onClick={() => setLightMode(!lightMode)}>{lightModeText}</button>
				</div>

				<form
					className="flex justify-center bg-[#113355] text-[white] px-4 py-2 rounded-md"
					onSubmit={(e) => {
						e.preventDefault();
						console.log("do some calls to the backend here!", username, bio);
					}}
				>
					<label className="px-1"> Username: </label>
					<input
						className="bg-[#113355] px-1"
						value={username}
						onChange={(e) => setUsername(e.currentTarget.value)}
					/>

					<label className="px-1"> Username: </label>
					<textarea
						className="bg-[#113355] px-1 resize-none"
						rows={5}
						value={bio}
						onChange={(e) => setBio(e.target.value)}
					/>

					<input id="submit" type="submit" className="px-1 bg-black rounded-md" value="Submit" />
				</form>
			</div>
		</div>
	);
}
