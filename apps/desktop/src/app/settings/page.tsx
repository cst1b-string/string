// Profile display name, bio?, dark/light mode, clear chats, etc.

'use client'

{/* <button onClick={() => false}>
<h1> Change Username: {Username} </h1>
</button> */}

import { useContext } from 'react';
import { themeContext } from '../page';

export default function Home() {

	const lightModeComposite = useContext(themeContext);
	const lightMode = lightModeComposite.lightMode;
	var lightModeText;
	var Username = "<Username from Tauri>";
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

					<button onClick={() => lightModeComposite.setLightMode(true)}>
						<h1> {lightModeText} </h1>
					</button>

				</div>

				<div className="flex justify-center bg-[#113355] text-[white] px-4 py-2 rounded-md">

					<h1 className="px-1"> Username: </h1>
					<input className="bg-[#113355] px-1" defaultValue={Username}/>
					
					<button className="px-1 bg-black rounded-md" onClick={() => false}>
						<h1> Submit </h1>
					</button>

				</div>

				<div className="flex justify-center bg-[#113355] text-[white] px-4 py-2 rounded-md">

					<h1 className="px-1"> User Bio: </h1>
					<textarea className="bg-[#113355] px-1 resize-none" rows={5} defaultValue={Bio}/>

					<button className="px-1 py-5 bg-black rounded-md" onClick={() => false}>
						<h1> Submit </h1>
					</button>

				</div>

			</div>
		</div>
	);
}
