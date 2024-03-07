"use client";

import { useRspc } from "@/integration";
import { useEffect, useMemo, useState } from "react";

export default function Home() {
	const rspc = useRspc();

	const { data, refetch } = rspc.useQuery(["settings.theme"]);
	const { mutate: themeMutate, isSuccess: themeIsSuccess } = rspc.useMutation(["settings.theme"]); //To send updated theme to backend

	useEffect(() => {
		refetch();
	}, [themeIsSuccess]); //Refetch on successful backend call

	const lightModeText = useMemo(() => (data === "Light" ? "Dark Mode" : "Light Mode"), [data]); //Text to be displayed on theme button

	const user_id = rspc.useQuery(["account.fingerprint_in_bytes"]).data;
	const curr_user = rspc.useQuery(["user.user", user_id]).data;

	const [username, setUsername] = useState(curr_user?.username);
	const [biography, setBiography] = useState(curr_user?.biography);

	const update_user_details = rspc.useMutation(["user.update_user_details"]).mutate;

	return (
		<div className="flex flex-row justify-center py-5">
			<div className="flex flex-col space-y-4 w-[600px]">
				<div className="flex  justify-center bg-darkSidebar text-darkText px-4 py-2 rounded-md cursor-pointer hover:bg-darkSidebar">

					<button onClick={() => themeMutate(data === "Dark" ? "Light" : "Dark")}>
						{lightModeText}
					</button>
				</div>

				<form
					className="flex justify-center flex-col space-y-4"
					onSubmit={(e) => {
						e.preventDefault();
						if (user_id && username && biography){
							update_user_details({user_id: user_id, username: username, biography: biography})
						}
					}}
				>
					<div className="flex items-center justify-center bg-darkSidebar text-darkText px-4 py-2 rounded-md">
						<label className="mr-4"> Username</label>
						<input
							className="flex-grow bg-darkBackground p-4 rounded-md"
							value={username}
							onChange={(e) => setUsername(e.currentTarget.value)}
						/>
					</div>

					<div className="flex flex-col justify-center bg-darkSidebar text-darkText px-4 py-2 rounded-md">
						<label className="py-4"> Biography</label>
						<textarea
							className="p-4 resize-none w-full rounded-md bg-darkBackground text-darkText"
							rows={5}
							value={biography}
							onChange={(e) => setBiography(e.target.value)}
						/>
					</div>

					<input
						id="submit"
						type="submit"
						className="flex justify-center bg-darkSidebar text-darkText px-4 py-2 rounded-md cursor-pointer hover:bg-darkSidebar"
						value="Submit"
					/>
				</form>
			</div>
		</div>
	);
}
