// Profile display name, bio?, dark/light mode, clear chats, etc.

'use client'

{/* <button onClick={() => false}>
<h1> Change Username: {Username} </h1>
</button> */}

import { useContext, useState } from 'react';
import { themeContext } from '../page';

const [usernameState, setUsernameState] = useState('');

export function set_username(username: string) {} //placeholder

export function handleUsernameSubmit() {
	set_username(usernameState);
}

export function handleUsernameChange(e: React.FormEvent<HTMLInputElement>) {

	setUsernameState(e.currentTarget.value);

}

export function set_bio(bio: string) {} //placeholder

export function handleBioSubmit() {
	set_username(usernameState);
}

export function handleBioChange(e: React.FormEvent<HTMLTextAreaElement>) {

	setUsernameState(e.currentTarget.value);

}

export function toggleLightMode(lightMode: boolean) {
	//backend call
}

export default function Home() {

	const lightModeComposite = useContext(themeContext);
	const lightMode = lightModeComposite.lightMode;
	var lightModeText;
	var Username = "<Username from Tauri>";

	setUsernameState(Username);

	var Bio = "<Bio from Tauri>";
	if (lightMode) {
		lightModeText = "Dark Mode"
	}

	else {
		lightModeText = "Light Mode"
	}
	return (
		<div className="flex justify-center">
			<div className="py-5 w-1/3 space-y-1">
				<div className="flex justify-center bg-[#113355] text-[white] px-4 py-2 rounded-md">

					<button onClick={() => toggleLightMode(!lightMode)}>
						<h1> {lightModeText} </h1>
					</button>

				</div>

				<div className="flex justify-center bg-[#113355] text-[white] px-4 py-2 rounded-md">

					<h1 className="px-1"> Username: </h1>
					<input className="bg-[#113355] px-1" defaultValue={Username} onChange={handleUsernameChange}/>
					
					<button className="px-1 bg-black rounded-md" onClick={handleUsernameSubmit}>
						<h1> Submit </h1>
					</button>

				</div>

				<div className="flex justify-center bg-[#113355] text-[white] px-4 py-2 rounded-md">

					<h1 className="px-1"> User Bio: </h1>
					<textarea className="bg-[#113355] px-1 resize-none" rows={5} defaultValue={Bio} onChange={handleBioChange}/>

					<button className="px-1 py-5 bg-black rounded-md" onClick={() => false}>
						<h1> Submit </h1>
					</button>

				</div>

			</div>
		</div>
	);
}
