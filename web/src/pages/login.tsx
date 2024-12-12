import { email, required, SubmitHandler, useForm } from "@modular-forms/react";
import * as v from "valibot";

const SignInSchema = v.object({
	email: v.pipe(v.string(), v.email()),
});

type SignInForm = v.InferOutput<typeof SignInSchema>;

export default function Login() {
	return (
		<div>
			<h1>Login</h1>
			<LoginForm />
		</div>
	);
}

function LoginForm() {
	const [_, { Form, Field }] = useForm<SignInForm>();

	const handleSubmit: SubmitHandler<SignInForm> = (values) => {
		console.log(values);
	};

	return (
		<Form onSubmit={handleSubmit}>
			<Field
				name="email"
				validate={[
					required("Please enter your email"),
					email("This email address is badly formatted"),
				]}
			>
				{(field, props) => (
					<>
						<input {...props} type="email" />
						{field.error && <div>{field.error.value}</div>}
					</>
				)}
			</Field>
			<button type="submit">Sign In</button>
		</Form>
	);
}
