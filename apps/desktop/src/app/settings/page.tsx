"use client";

import { useContext, useMemo, useState } from "react";

import { ThemeContext } from "../layout";

import { useRspc } from "@/integration";

export default function Home() {

	const rspc = useRspc();
	const theme = rspc.useQuery(["settings.theme"]);

	var initial_theme;

	if (theme.data == "Light") {
		initial_theme = true;
	}
	else {
		initial_theme = false;
	}

	const [lightMode, setLightMode] = useState(initial_theme);

	//const { lightMode, setLightMode } = useContext(ThemeContext);

	const lightModeText = useMemo(() => (lightMode ? "Dark Mode" : "Light Mode"), [lightMode]);

	const [username, setUsername] = useState("<Username from Tauri>");
	const [bio, setBio] = useState("<Bio from Tauri>");

	return (
		<div className="flex flex-row justify-center py-5">
			<div className="flex flex-col space-y-4 w-[600px]">
				<div className="flex  justify-center bg-[#335577] text-[white] px-4 py-2 rounded-md cursor-pointer hover:bg-[#224466]">
					<button onClick={() => setLightMode(!lightMode)}>{lightModeText}</button>
				</div>

				<form
					className="flex justify-center flex-col space-y-4"
					onSubmit={(e) => {
						e.preventDefault();
						console.log("do some calls to the backend here!", username, bio);
					}}
				>
					<div className="flex items-center justify-center bg-[#113355] text-[white] px-4 py-2 rounded-md">
						<label className="mr-4"> Username</label>
						<input
							className="flex-grow bg-[#335577] p-4 rounded-md"
							value={username}
							onChange={(e) => setUsername(e.currentTarget.value)}
						/>
					</div>

					<div className="flex flex-col justify-center bg-[#113355] text-[white] px-4 py-2 rounded-md">
						<label className="py-4"> Biography</label>
						<textarea
							className="p-4 resize-none w-full rounded-md bg-[#335577]"
							rows={5}
							value={bio}
							onChange={(e) => setBio(e.target.value)}
						/>
					</div>

					<input
						id="submit"
						type="submit"
						className="flex justify-center bg-[#335577] text-[white] px-4 py-2 rounded-md cursor-pointer hover:bg-[#224466]"
						value="Submit"
					/>
				</form>
			</div>
		</div>
	);
}
