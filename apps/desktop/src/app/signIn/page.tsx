'use client'

import React, { useState } from 'react';

export default function SignIn() {
	const [username, setUsername] = useState('');
	const [password, setPassword] = useState('');

	const handleUsernameChange = (event: React.ChangeEvent<HTMLInputElement>) => {
		setUsername(event.target.value);
	};

	const handlePasswordChange = (event: React.ChangeEvent<HTMLInputElement>) => {
		setPassword(event.target.value);
	};

	const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		// Perform sign-in logic here
	};

	return (
		<div>
			<form onSubmit={handleSubmit}>
				<label>
					Username:
					<input type="text" value={username} onChange={handleUsernameChange} />
				</label>
				<br />
				<label>
					Password:
					<input type="password" value={password} onChange={handlePasswordChange} />
				</label>
				<br />
				<button type="submit">Sign In</button>
			</form>
		</div>
	);
}
