--
-- PostgreSQL database dump
--

\restrict J0cMhaf3dsyG0jO2Duoi5ekR4bma3oi0z1obBIPxCTARl0cC9JGhocZbJHblaLN

-- Dumped from database version 17.6
-- Dumped by pg_dump version 18.4 (Homebrew)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Data for Name: archive_manifest; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.archive_manifest (id, org_id, table_name, object_key, row_count, archived_at) FROM stdin;
\.


--
-- Data for Name: audit_log; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.audit_log (id, org_id, actor_id, action, entity_type, entity_id, before, after, request_id, created_at) FROM stdin;
1	32f33aef-dc6a-4caf-9f05-794f65f30c8a	00e1e1f4-96d0-43ec-b27e-e5cc04297ffa	create	branch	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	\N	{"id": "af1202c3-9efe-4de3-a5ee-ce7383b81e0b", "name": "Main Branch", "org_id": "32f33aef-dc6a-4caf-9f05-794f65f30c8a", "address": null, "created_at": "2026-08-11T07:57:24.539473+00:00", "deleted_at": null}	\N	2026-08-11 07:57:24.539473+00
16	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	branch	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	\N	{"name": "HSR Layout"}	c964bc15-8413-03e8-035e-ce803213aedc	2026-01-05 04:05:00+00
17	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	e918a2e5-f700-12d1-5cd4-28f8a5340024	\N	{"number": "102", "cooling_type": "ac"}	67d9ad0d-0aaa-0188-7745-74aa2c126f68	2026-01-05 04:10:00+00
18	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	ed724e26-59f9-9647-6a98-f39f40be3e2a	\N	{"label": "A"}	599f5819-d76b-a0f9-e378-1ca9e5c6c703	2026-01-05 04:15:00+00
19	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	move_out	resident	b90bbc07-df8e-3bed-1d7d-7c245e23912a	\N	{"last_day": "2026-05-31", "resident": "Naveen Rao"}	28a155fc-4140-1bb0-04e3-c7156084ebc7	2026-06-02 10:30:00+00
20	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	activate_tenant	resident	b90bbc07-df8e-3bed-1d7d-7c245e23912a	\N	{"role": "tenant", "invitation": "accepted"}	c274e00b-68b0-b121-3a95-b560f276f4bc	2026-05-26 06:45:00+00
21	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	link_user	resident	b90bbc07-df8e-3bed-1d7d-7c245e23912a	\N	{"linked": "kriahkunar2@gmail.com"}	6401a9cd-15fe-417b-3b89-2b3bec983be1	2026-05-26 06:45:05+00
22	cb68eae7-2955-5cae-e440-0a0190731fe1	0f2c58a3-c922-48ef-9571-68c5eb743a87	assign_bed	resident	b90bbc07-df8e-3bed-1d7d-7c245e23912a	\N	{"bed": "A", "room": "102"}	2ab1e6f9-e98b-50e9-b502-47d0fb6eb7a9	2026-06-01 05:30:00+00
23	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	payment	payment	672a55a0-251a-2561-6a09-2a289efc2491	\N	{"method": "upi", "amount_paise": 1250000}	19c3f15c-980f-e383-7e28-24a648852944	2026-06-05 13:00:00+00
24	cb68eae7-2955-5cae-e440-0a0190731fe1	0f2c58a3-c922-48ef-9571-68c5eb743a87	payment	payment	63b596f9-aee9-e9a4-4cc0-6c0dff331913	\N	{"method": "cash", "amount_paise": 1250000}	b861759e-fde3-dadf-f455-7564f0238d72	2026-07-04 13:45:00+00
25	cb68eae7-2955-5cae-e440-0a0190731fe1	0f2c58a3-c922-48ef-9571-68c5eb743a87	create	complaint	b20fc105-a806-ef04-be73-e107cce6729a	\N	{"title": "Wi-Fi drops between 9pm and 11pm"}	60ab1ed7-26c1-4bc0-1103-7f25ef93d0a5	2026-07-18 16:00:00+00
26	cb68eae7-2955-5cae-e440-0a0190731fe1	0f2c58a3-c922-48ef-9571-68c5eb743a87	complaint_transition	complaint	004972ae-d2dc-fd52-59c9-601da65a83fc	\N	{"to": "in_progress", "from": "assigned"}	18083691-56b3-4b4c-8e07-0121bef7a5c8	2026-08-04 05:50:00+00
27	cb68eae7-2955-5cae-e440-0a0190731fe1	0f2c58a3-c922-48ef-9571-68c5eb743a87	create	notice	453d040d-1b28-4995-0c2d-973dc9e1e61e	\N	{"title": "Water supply maintenance on 14 Aug"}	759abb55-dcdf-9b01-996d-2b25fa1822c0	2026-08-08 12:50:00+00
28	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	payment	payment	79ec5038-7b4d-4609-ac0b-040aaa3c9c3c	\N	\N	c8be1bd3-5d41-4f49-97c4-1c24eec8fe6e	2026-08-12 17:39:28.027079+00
29	cb68eae7-2955-5cae-e440-0a0190731fe1	0f2c58a3-c922-48ef-9571-68c5eb743a87	payment	payment	1b6d5372-a3fc-4b1b-a386-64e93be27886	\N	\N	0447eba3-4b8e-4423-8385-b927e2157bb0	2026-08-12 19:29:19.112453+00
30	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	payment_void	payment	1b6d5372-a3fc-4b1b-a386-64e93be27886	\N	{"resident_id": "b90bbc07-df8e-3bed-1d7d-7c245e23912a", "amount_paise": 100000, "invoice_number": "INV-2608-003"}	c0714c22-ef29-4b43-a629-7a95bdd2647a	2026-08-12 19:29:34.035004+00
31	cb68eae7-2955-5cae-e440-0a0190731fe1	92b0cc9d-1633-cbde-3356-6c31054360b9	cancel	away_period	5bbe78c4-e741-9301-0bb0-e7abe8705b9a	{"id": "5bbe78c4-e741-9301-0bb0-e7abe8705b9a", "note": "Travelling home for Onam", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "status": "active", "to_date": "2026-08-25", "from_date": "2026-08-20", "created_at": "2026-08-09T14:45:00+00:00", "deleted_at": null, "resident_id": "b90bbc07-df8e-3bed-1d7d-7c245e23912a", "cancelled_at": null}	{"id": "5bbe78c4-e741-9301-0bb0-e7abe8705b9a", "note": "Travelling home for Onam", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "status": "cancelled", "to_date": "2026-08-25", "from_date": "2026-08-20", "created_at": "2026-08-09T14:45:00+00:00", "deleted_at": null, "resident_id": "b90bbc07-df8e-3bed-1d7d-7c245e23912a", "cancelled_at": "2026-08-15T22:41:40.702433+00:00"}	933f1dc7-f43d-44ec-85f7-6d4a1c18271f	2026-08-15 22:41:40.702433+00
32	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	activate_tenant	resident	2d83dfa8-338e-44da-bc3c-c864ac397d74	\N	{"invitation_id": "ebef1e71-90dc-420d-8c25-7991cd78d08d", "membership_id": "abd1ef33-d563-41bb-a4d2-4bc89fc79b0a"}	1c75f701-af4b-4fe1-b0d1-c25b3fd9b51a	2026-08-31 14:41:28.708912+00
33	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	activate_tenant	resident	ec7cf3d5-a9d9-47ac-9f00-9b312635a175	\N	{"invitation_id": "edc84046-5779-4c71-b6b6-4fc8dc66643a", "membership_id": "9ec2cde0-6a8e-455b-b4b7-1dea2105b9ad"}	c3f74b4a-aba2-470a-bf3d-628b83abc310	2026-08-31 15:13:57.099711+00
34	5d49310f-864e-4a65-926b-f4168d1dd9b6	3d597748-da72-422a-b29d-3d104dd1ec9a	create	branch	f67789c7-edd7-4494-a6fa-3b6fc82e5f28	\N	{"id": "f67789c7-edd7-4494-a6fa-3b6fc82e5f28", "name": "Main Branch", "org_id": "5d49310f-864e-4a65-926b-f4168d1dd9b6", "address": null, "created_at": "2026-09-01T06:21:54.990901+00:00", "deleted_at": null}	\N	2026-09-01 06:21:54.990901+00
35	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	payment	payment	70899a74-1a00-4d4d-a91a-6601d2f42d77	\N	\N	b53011b1-2d0c-41ce-b7cb-d8698d3c65a9	2026-09-01 10:05:32.363975+00
36	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	payment_void	payment	70899a74-1a00-4d4d-a91a-6601d2f42d77	\N	{"resident_id": "2d83dfa8-338e-44da-bc3c-c864ac397d74", "amount_paise": 41666, "invoice_number": "INV-2609-001"}	0c3145d1-0819-43f8-8744-db151dff3c10	2026-09-01 10:05:43.744808+00
37	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	payment	payment	a1ff2dfe-3b26-4c65-aa38-3634e1ccdefb	\N	\N	22ec57c1-268a-46d1-8340-42b18ac38b85	2026-09-01 10:05:49.84756+00
38	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	floor	1114b103-e9e5-4ac6-af74-e02c5b62016c	\N	{"id": "1114b103-e9e5-4ac6-af74-e02c5b62016c", "name": "Floor 1", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "created_at": "2026-09-01T19:47:42.870435+00:00", "deleted_at": null, "sort_order": 1}	e82d044f-55e2-4c93-979d-e7c0efc8fdb7	2026-09-01 19:47:42.870435+00
39	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	floor	e5c3ce85-2e17-446f-ab2d-4d824b1cefad	\N	{"id": "e5c3ce85-2e17-446f-ab2d-4d824b1cefad", "name": "Floor 2", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "created_at": "2026-09-01T19:47:43.55756+00:00", "deleted_at": null, "sort_order": 2}	e582366a-3365-491b-988a-4a01967044df	2026-09-01 19:47:43.55756+00
40	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	update	room	e3beab34-30cc-d1b5-b04d-2f85fdc035ba	{"id": "e3beab34-30cc-d1b5-b04d-2f85fdc035ba", "number": "101", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": null, "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "created_at": "2026-01-05T04:10:00+00:00", "deleted_at": null, "cooling_type": "non_ac"}	{"id": "e3beab34-30cc-d1b5-b04d-2f85fdc035ba", "number": "101", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "1114b103-e9e5-4ac6-af74-e02c5b62016c", "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "created_at": "2026-01-05T04:10:00+00:00", "deleted_at": null, "cooling_type": "non_ac"}	1ea16ded-ca99-446a-8775-837ffa65b6ae	2026-09-01 19:47:59.596389+00
41	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	payment	payment	39612637-41c6-4aa5-9d2e-8d009d778bef	\N	\N	5e085607-96fa-46d5-abd9-08f9aa37a548	2026-09-02 03:23:40.536377+00
42	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	payment	payment	f153e0a1-1423-4924-8314-0176adbccbad	\N	\N	d50cc1c1-349e-40d5-9e76-f25310bcf8a0	2026-09-02 03:23:46.078808+00
43	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	payment	payment	f69f3931-15a6-4eca-b2fb-fc2379906d3d	\N	\N	0def828b-4cc2-4633-bb24-45800bf6a297	2026-09-02 03:27:46.395162+00
44	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	payment	payment	29ddb942-31de-420a-9a11-ce366f842f1a	\N	\N	5e89f95f-52f4-4502-b9d5-bf39b8daa4f8	2026-09-02 03:27:49.882221+00
45	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	payment	payment	88a9e699-38d5-4900-8bc1-b2fdf0e6a32a	\N	\N	e8614b24-37e5-4315-8c0b-7ec549ac7ebe	2026-09-02 03:31:36.149547+00
46	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	payment	payment	84533728-d4a5-443c-a720-ac8d8b886a27	\N	\N	a5802e09-c5bf-487a-a250-fa981ac5e606	2026-09-02 03:52:53.284166+00
47	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	c710ff76-047f-4a70-bd15-a3159560daaf	\N	{"id": "c710ff76-047f-4a70-bd15-a3159560daaf", "number": "999", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": null, "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "created_at": "2026-09-02T04:11:57.845452+00:00", "deleted_at": null, "cooling_type": "ac"}	cc68cc7d-4dff-4853-afea-066b831b7053	2026-09-02 04:11:57.845452+00
48	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	update	room	e918a2e5-f700-12d1-5cd4-28f8a5340024	{"id": "e918a2e5-f700-12d1-5cd4-28f8a5340024", "number": "102", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": null, "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "created_at": "2026-01-05T04:10:00+00:00", "deleted_at": null, "cooling_type": "ac"}	{"id": "e918a2e5-f700-12d1-5cd4-28f8a5340024", "number": "102", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "e5c3ce85-2e17-446f-ab2d-4d824b1cefad", "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "created_at": "2026-01-05T04:10:00+00:00", "deleted_at": null, "cooling_type": "ac"}	ad0f0b35-ee73-4908-af55-af5e4190f445	2026-09-02 04:12:16.401181+00
49	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	update	room	e918a2e5-f700-12d1-5cd4-28f8a5340024	{"id": "e918a2e5-f700-12d1-5cd4-28f8a5340024", "number": "102", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "e5c3ce85-2e17-446f-ab2d-4d824b1cefad", "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "created_at": "2026-01-05T04:10:00+00:00", "deleted_at": null, "cooling_type": "ac"}	{"id": "e918a2e5-f700-12d1-5cd4-28f8a5340024", "number": "102", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "e5c3ce85-2e17-446f-ab2d-4d824b1cefad", "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "created_at": "2026-01-05T04:10:00+00:00", "deleted_at": null, "cooling_type": "ac"}	377ef4ec-7ca2-4b7e-94fe-f73b8344e22a	2026-09-02 04:12:17.127661+00
50	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	floor	f44c738a-cbc4-4c61-85a7-dfe600d4be93	\N	{"id": "f44c738a-cbc4-4c61-85a7-dfe600d4be93", "name": "Floor 3", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "created_at": "2026-09-02T04:34:34.738971+00:00", "deleted_at": null, "sort_order": 3}	4ea993e2-267f-466a-a7f3-96d16e6c6647	2026-09-02 04:34:34.738971+00
51	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	floor	b7b5e3ff-175b-42d9-a727-cab68c62b6b4	\N	{"id": "b7b5e3ff-175b-42d9-a727-cab68c62b6b4", "name": "Floor 4", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "created_at": "2026-09-02T04:34:54.832393+00:00", "deleted_at": null, "sort_order": 4}	4bb9acdc-fb3d-4b36-8c24-6ae8a0d7e681	2026-09-02 04:34:54.832393+00
52	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	298afa66-6b9f-4fdc-946f-a0c86e079052	\N	{"id": "298afa66-6b9f-4fdc-946f-a0c86e079052", "number": "301", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": null, "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "created_at": "2026-09-02T04:34:55.782246+00:00", "deleted_at": null, "cooling_type": "ac"}	f0c321cb-1a9e-4e7a-acf9-74f3719d5062	2026-09-02 04:34:55.782246+00
53	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	delete	floor	b7b5e3ff-175b-42d9-a727-cab68c62b6b4	{"id": "b7b5e3ff-175b-42d9-a727-cab68c62b6b4", "name": "Floor 4", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "created_at": "2026-09-02T04:34:54.832393+00:00", "deleted_at": null, "sort_order": 4}	\N	b85b4056-2100-4940-bc4d-37861c915eab	2026-09-02 04:35:20.302873+00
54	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	branch	36030a1c-ec16-4451-bc29-87672e9cbeb6	\N	{"id": "36030a1c-ec16-4451-bc29-87672e9cbeb6", "name": "Seed Test Branch", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "address": null, "created_at": "2026-09-02T04:37:28.613669+00:00", "deleted_at": null}	e66660eb-10af-44ee-936c-547a565d78a6	2026-09-02 04:37:28.613669+00
55	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	branch	72a49ed9-6a5f-4378-b601-a5f309c1d1e3	\N	{"id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "name": "Seed Test 2", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "address": null, "created_at": "2026-09-02T04:38:24.282576+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:24.282576+00
56	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	floor	299240d3-6763-4c39-a59f-181c7ceb7564	\N	{"id": "299240d3-6763-4c39-a59f-181c7ceb7564", "name": "Floor 1", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:25.753342+00:00", "deleted_at": null, "sort_order": 1}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:25.753342+00
57	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	215927f8-fdd4-4156-887b-a468b9512df6	\N	{"id": "215927f8-fdd4-4156-887b-a468b9512df6", "number": "101", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "299240d3-6763-4c39-a59f-181c7ceb7564", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:26.270136+00:00", "deleted_at": null, "cooling_type": "ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:26.270136+00
58	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	bf750ae7-0a95-454c-a701-2369704db4ef	\N	{"id": "bf750ae7-0a95-454c-a701-2369704db4ef", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "215927f8-fdd4-4156-887b-a468b9512df6", "created_at": "2026-09-02T04:38:27.187284+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:27.187284+00
59	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	1ba7d6db-d15d-4aa1-a57d-d4210a523a15	\N	{"id": "1ba7d6db-d15d-4aa1-a57d-d4210a523a15", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "215927f8-fdd4-4156-887b-a468b9512df6", "created_at": "2026-09-02T04:38:27.748018+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:27.748018+00
60	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	9febbb57-2925-4c92-b376-afb4c8b0d8ce	\N	{"id": "9febbb57-2925-4c92-b376-afb4c8b0d8ce", "number": "102", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "299240d3-6763-4c39-a59f-181c7ceb7564", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:28.259796+00:00", "deleted_at": null, "cooling_type": "ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:28.259796+00
61	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	b45c6aeb-0724-42ad-b427-d60570703cac	\N	{"id": "b45c6aeb-0724-42ad-b427-d60570703cac", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "9febbb57-2925-4c92-b376-afb4c8b0d8ce", "created_at": "2026-09-02T04:38:29.107875+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:29.107875+00
65	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	b6e8cc40-07f7-4c17-9706-dfa204f12fb7	\N	{"id": "b6e8cc40-07f7-4c17-9706-dfa204f12fb7", "number": "103", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "299240d3-6763-4c39-a59f-181c7ceb7564", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:30.930842+00:00", "deleted_at": null, "cooling_type": "non_ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:30.930842+00
68	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	7ab20e4d-dd52-4c96-8ad4-c2000f939cbf	\N	{"id": "7ab20e4d-dd52-4c96-8ad4-c2000f939cbf", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "b6e8cc40-07f7-4c17-9706-dfa204f12fb7", "created_at": "2026-09-02T04:38:32.613902+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:32.613902+00
71	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	36078a39-0908-4d18-b666-a26e0bcfab51	\N	{"id": "36078a39-0908-4d18-b666-a26e0bcfab51", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "52f812eb-db37-4858-ae95-fc8005c73ac7", "created_at": "2026-09-02T04:38:34.235143+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:34.235143+00
75	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	66b5e4c9-8151-453a-93dd-67a4657b4720	\N	{"id": "66b5e4c9-8151-453a-93dd-67a4657b4720", "label": "E", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "52f812eb-db37-4858-ae95-fc8005c73ac7", "created_at": "2026-09-02T04:38:35.829472+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:35.829472+00
78	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	720e00df-416a-4abb-838e-cb3a88a6c077	\N	{"id": "720e00df-416a-4abb-838e-cb3a88a6c077", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "fc9f1b33-8459-43cd-9065-327b0ea03080", "created_at": "2026-09-02T04:38:37.353249+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:37.353249+00
81	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	e5167fb5-3220-4d27-894a-aa11695cc739	\N	{"id": "e5167fb5-3220-4d27-894a-aa11695cc739", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "ccd338fe-0e6e-42f2-b3a0-c8e0b352c07d", "created_at": "2026-09-02T04:38:38.797956+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:38.797956+00
85	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	38b753d6-5cc7-4fc4-b414-ecb01d8a11ce	\N	{"id": "38b753d6-5cc7-4fc4-b414-ecb01d8a11ce", "number": "301", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "31838d43-efb5-4caf-aa26-d86df256e4c2", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:40.400663+00:00", "deleted_at": null, "cooling_type": "non_ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:40.400663+00
88	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	defc998a-7a78-408f-9259-f67fbea680d6	\N	{"id": "defc998a-7a78-408f-9259-f67fbea680d6", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "38b753d6-5cc7-4fc4-b414-ecb01d8a11ce", "created_at": "2026-09-02T04:38:41.843075+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:41.843075+00
91	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	ef592f7f-8d63-486d-ab89-8d53db1f9a9c	\N	{"id": "ef592f7f-8d63-486d-ab89-8d53db1f9a9c", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "1128ed3a-fd25-4597-9327-e75fd188229c", "created_at": "2026-09-02T04:38:43.339636+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:43.339636+00
94	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	8974c546-e27c-45ba-96c8-1c6b97d401c4	\N	{"id": "8974c546-e27c-45ba-96c8-1c6b97d401c4", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "de57965d-8f54-46f9-92c2-a0c1553aae10", "created_at": "2026-09-02T04:38:44.801641+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:44.801641+00
98	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	38acb91c-d1cf-4449-84a5-afc204eb36ea	\N	{"id": "38acb91c-d1cf-4449-84a5-afc204eb36ea", "number": "401", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "66cc4759-e48c-4369-9c9d-f0e3c0f5b5d0", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:46.298842+00:00", "deleted_at": null, "cooling_type": "non_ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:46.298842+00
101	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	43ccadb0-5dea-4d2f-9382-57ed622f39a3	\N	{"id": "43ccadb0-5dea-4d2f-9382-57ed622f39a3", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "38acb91c-d1cf-4449-84a5-afc204eb36ea", "created_at": "2026-09-02T04:38:47.717865+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:47.717865+00
104	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	54b79d11-6791-42e7-bdf7-4addd9a6ff98	\N	{"id": "54b79d11-6791-42e7-bdf7-4addd9a6ff98", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "ea28ffe6-ee57-4553-af1d-d25ab6d1494d", "created_at": "2026-09-02T04:38:49.243794+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:49.243794+00
62	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	0131810b-af13-4794-89f7-93f7cdff736c	\N	{"id": "0131810b-af13-4794-89f7-93f7cdff736c", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "9febbb57-2925-4c92-b376-afb4c8b0d8ce", "created_at": "2026-09-02T04:38:29.511158+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:29.511158+00
69	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	floor	7702a888-72ca-446b-bb68-42276346202a	\N	{"id": "7702a888-72ca-446b-bb68-42276346202a", "name": "Floor 2", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:33.021808+00:00", "deleted_at": null, "sort_order": 2}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:33.021808+00
72	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	f1119c84-0cc4-40cb-932a-de09209aa2f3	\N	{"id": "f1119c84-0cc4-40cb-932a-de09209aa2f3", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "52f812eb-db37-4858-ae95-fc8005c73ac7", "created_at": "2026-09-02T04:38:34.608739+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:34.608739+00
76	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	fc9f1b33-8459-43cd-9065-327b0ea03080	\N	{"id": "fc9f1b33-8459-43cd-9065-327b0ea03080", "number": "202", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "7702a888-72ca-446b-bb68-42276346202a", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:36.196936+00:00", "deleted_at": null, "cooling_type": "non_ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:36.196936+00
79	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	02a69a07-5129-4f86-a967-3088b38327ac	\N	{"id": "02a69a07-5129-4f86-a967-3088b38327ac", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "fc9f1b33-8459-43cd-9065-327b0ea03080", "created_at": "2026-09-02T04:38:37.740892+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:37.740892+00
82	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	44e12ec1-288f-481d-a027-3a4d2e6da760	\N	{"id": "44e12ec1-288f-481d-a027-3a4d2e6da760", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "ccd338fe-0e6e-42f2-b3a0-c8e0b352c07d", "created_at": "2026-09-02T04:38:39.210333+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:39.210333+00
89	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	1128ed3a-fd25-4597-9327-e75fd188229c	\N	{"id": "1128ed3a-fd25-4597-9327-e75fd188229c", "number": "302", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "31838d43-efb5-4caf-aa26-d86df256e4c2", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:42.239272+00:00", "deleted_at": null, "cooling_type": "non_ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:42.239272+00
92	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	649409a9-acd4-46f9-b011-39a1b5f9b267	\N	{"id": "649409a9-acd4-46f9-b011-39a1b5f9b267", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "1128ed3a-fd25-4597-9327-e75fd188229c", "created_at": "2026-09-02T04:38:43.718765+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:43.718765+00
95	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	ed133df2-5185-49e1-a678-d6a7e19151f0	\N	{"id": "ed133df2-5185-49e1-a678-d6a7e19151f0", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "de57965d-8f54-46f9-92c2-a0c1553aae10", "created_at": "2026-09-02T04:38:45.172015+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:45.172015+00
102	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	ea28ffe6-ee57-4553-af1d-d25ab6d1494d	\N	{"id": "ea28ffe6-ee57-4553-af1d-d25ab6d1494d", "number": "402", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "66cc4759-e48c-4369-9c9d-f0e3c0f5b5d0", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:48.1222+00:00", "deleted_at": null, "cooling_type": "non_ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:48.1222+00
63	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	f9423366-516c-4656-b475-0a128057f8be	\N	{"id": "f9423366-516c-4656-b475-0a128057f8be", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "9febbb57-2925-4c92-b376-afb4c8b0d8ce", "created_at": "2026-09-02T04:38:29.899124+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:29.899124+00
66	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	47477b25-3ef0-4d66-a70b-889758c7aed7	\N	{"id": "47477b25-3ef0-4d66-a70b-889758c7aed7", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "b6e8cc40-07f7-4c17-9706-dfa204f12fb7", "created_at": "2026-09-02T04:38:31.793988+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:31.793988+00
70	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	52f812eb-db37-4858-ae95-fc8005c73ac7	\N	{"id": "52f812eb-db37-4858-ae95-fc8005c73ac7", "number": "201", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "7702a888-72ca-446b-bb68-42276346202a", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:33.533217+00:00", "deleted_at": null, "cooling_type": "non_ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:33.533217+00
73	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	4757dfae-eca0-4cbc-942e-8fe5634c455d	\N	{"id": "4757dfae-eca0-4cbc-942e-8fe5634c455d", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "52f812eb-db37-4858-ae95-fc8005c73ac7", "created_at": "2026-09-02T04:38:35.010466+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:35.010466+00
80	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	ccd338fe-0e6e-42f2-b3a0-c8e0b352c07d	\N	{"id": "ccd338fe-0e6e-42f2-b3a0-c8e0b352c07d", "number": "203", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "7702a888-72ca-446b-bb68-42276346202a", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:38.145838+00:00", "deleted_at": null, "cooling_type": "non_ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:38.145838+00
83	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	9a2321f0-7fb2-4d1c-a2ff-e1dacaf67028	\N	{"id": "9a2321f0-7fb2-4d1c-a2ff-e1dacaf67028", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "ccd338fe-0e6e-42f2-b3a0-c8e0b352c07d", "created_at": "2026-09-02T04:38:39.618619+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:39.618619+00
86	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	724d6605-ab1d-426a-bb6b-bb660464304b	\N	{"id": "724d6605-ab1d-426a-bb6b-bb660464304b", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "38b753d6-5cc7-4fc4-b414-ecb01d8a11ce", "created_at": "2026-09-02T04:38:41.097956+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:41.097956+00
93	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	de57965d-8f54-46f9-92c2-a0c1553aae10	\N	{"id": "de57965d-8f54-46f9-92c2-a0c1553aae10", "number": "303", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "31838d43-efb5-4caf-aa26-d86df256e4c2", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:44.128681+00:00", "deleted_at": null, "cooling_type": "non_ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:44.128681+00
96	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	b09f5dd1-73e0-40c2-b55e-691f27aa66c4	\N	{"id": "b09f5dd1-73e0-40c2-b55e-691f27aa66c4", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "de57965d-8f54-46f9-92c2-a0c1553aae10", "created_at": "2026-09-02T04:38:45.564881+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:45.564881+00
99	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	2f040e93-7938-43ae-944e-ffcf3cd490cc	\N	{"id": "2f040e93-7938-43ae-944e-ffcf3cd490cc", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "38acb91c-d1cf-4449-84a5-afc204eb36ea", "created_at": "2026-09-02T04:38:46.95998+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:46.95998+00
64	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	d5fe68c5-19c4-4ffe-90de-b9ff203954f4	\N	{"id": "d5fe68c5-19c4-4ffe-90de-b9ff203954f4", "label": "D", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "9febbb57-2925-4c92-b376-afb4c8b0d8ce", "created_at": "2026-09-02T04:38:30.405676+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:30.405676+00
67	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	4dc16db9-74fe-41dc-bbf4-8d9eea1d5837	\N	{"id": "4dc16db9-74fe-41dc-bbf4-8d9eea1d5837", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "b6e8cc40-07f7-4c17-9706-dfa204f12fb7", "created_at": "2026-09-02T04:38:32.208858+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:32.208858+00
74	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	f8f32e00-f08a-4b3b-b4af-85a03b7ebcda	\N	{"id": "f8f32e00-f08a-4b3b-b4af-85a03b7ebcda", "label": "D", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "52f812eb-db37-4858-ae95-fc8005c73ac7", "created_at": "2026-09-02T04:38:35.420173+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:35.420173+00
77	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	2e3dcb4a-8efe-4649-88a7-5d5610fce23f	\N	{"id": "2e3dcb4a-8efe-4649-88a7-5d5610fce23f", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "fc9f1b33-8459-43cd-9065-327b0ea03080", "created_at": "2026-09-02T04:38:36.961514+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:36.961514+00
84	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	floor	31838d43-efb5-4caf-aa26-d86df256e4c2	\N	{"id": "31838d43-efb5-4caf-aa26-d86df256e4c2", "name": "Floor 3", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:39.98647+00:00", "deleted_at": null, "sort_order": 3}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:39.98647+00
87	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	d95cd6f9-35f4-4135-8392-06e2d37d959e	\N	{"id": "d95cd6f9-35f4-4135-8392-06e2d37d959e", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "38b753d6-5cc7-4fc4-b414-ecb01d8a11ce", "created_at": "2026-09-02T04:38:41.463137+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:41.463137+00
90	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	9dc3d468-2d6d-485b-9ca0-304e6b75fa21	\N	{"id": "9dc3d468-2d6d-485b-9ca0-304e6b75fa21", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "1128ed3a-fd25-4597-9327-e75fd188229c", "created_at": "2026-09-02T04:38:42.967343+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:42.967343+00
97	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	floor	66cc4759-e48c-4369-9c9d-f0e3c0f5b5d0	\N	{"id": "66cc4759-e48c-4369-9c9d-f0e3c0f5b5d0", "name": "Floor 4", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:45.934633+00:00", "deleted_at": null, "sort_order": 4}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:45.934633+00
100	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	253e83cc-89ed-40ff-8629-52bf17c97bf7	\N	{"id": "253e83cc-89ed-40ff-8629-52bf17c97bf7", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "38acb91c-d1cf-4449-84a5-afc204eb36ea", "created_at": "2026-09-02T04:38:47.356478+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:47.356478+00
103	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	f9d7bb78-a57c-4bd7-be08-fa660b8a1a71	\N	{"id": "f9d7bb78-a57c-4bd7-be08-fa660b8a1a71", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "ea28ffe6-ee57-4553-af1d-d25ab6d1494d", "created_at": "2026-09-02T04:38:48.838554+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:48.838554+00
105	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	a81b3314-728f-4d20-b52f-d31e910235a9	\N	{"id": "a81b3314-728f-4d20-b52f-d31e910235a9", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "ea28ffe6-ee57-4553-af1d-d25ab6d1494d", "created_at": "2026-09-02T04:38:49.656209+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:49.656209+00
106	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	5dffe5cb-503d-4955-ad46-bc95cb290ce4	\N	{"id": "5dffe5cb-503d-4955-ad46-bc95cb290ce4", "number": "403", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "66cc4759-e48c-4369-9c9d-f0e3c0f5b5d0", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:50.065991+00:00", "deleted_at": null, "cooling_type": "non_ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:50.065991+00
107	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	190fe6d2-fca7-47c4-bb14-7a83ac555416	\N	{"id": "190fe6d2-fca7-47c4-bb14-7a83ac555416", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "5dffe5cb-503d-4955-ad46-bc95cb290ce4", "created_at": "2026-09-02T04:38:50.735032+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:50.735032+00
108	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	458561d7-df25-4163-9453-1c94e97f1504	\N	{"id": "458561d7-df25-4163-9453-1c94e97f1504", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "5dffe5cb-503d-4955-ad46-bc95cb290ce4", "created_at": "2026-09-02T04:38:51.151944+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:51.151944+00
109	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	ef6aea7e-3c92-41a9-ba6a-5699521932c4	\N	{"id": "ef6aea7e-3c92-41a9-ba6a-5699521932c4", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "5dffe5cb-503d-4955-ad46-bc95cb290ce4", "created_at": "2026-09-02T04:38:51.559289+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:51.559289+00
110	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	floor	88f80763-7533-4291-94d5-a4ac46adbe3b	\N	{"id": "88f80763-7533-4291-94d5-a4ac46adbe3b", "name": "Floor 5", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:51.930904+00:00", "deleted_at": null, "sort_order": 5}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:51.930904+00
111	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	c84d937f-6b3c-4124-8ff0-4d8df4e73001	\N	{"id": "c84d937f-6b3c-4124-8ff0-4d8df4e73001", "number": "501", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "88f80763-7533-4291-94d5-a4ac46adbe3b", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:52.294959+00:00", "deleted_at": null, "cooling_type": "non_ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:52.294959+00
112	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	b7d329d3-cf69-4eb7-a4a9-78dfeb768bb2	\N	{"id": "b7d329d3-cf69-4eb7-a4a9-78dfeb768bb2", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "c84d937f-6b3c-4124-8ff0-4d8df4e73001", "created_at": "2026-09-02T04:38:52.991247+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:52.991247+00
113	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	d566354e-c6d9-4786-8a54-5f0bf498f053	\N	{"id": "d566354e-c6d9-4786-8a54-5f0bf498f053", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "c84d937f-6b3c-4124-8ff0-4d8df4e73001", "created_at": "2026-09-02T04:38:53.374576+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:53.374576+00
114	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	9d801034-e488-41ba-8b7f-6b2cbdece690	\N	{"id": "9d801034-e488-41ba-8b7f-6b2cbdece690", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "c84d937f-6b3c-4124-8ff0-4d8df4e73001", "created_at": "2026-09-02T04:38:53.752213+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:53.752213+00
115	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	b34aa434-7514-4794-873a-65bed73e8a6d	\N	{"id": "b34aa434-7514-4794-873a-65bed73e8a6d", "number": "502", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "88f80763-7533-4291-94d5-a4ac46adbe3b", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:54.12827+00:00", "deleted_at": null, "cooling_type": "non_ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:54.12827+00
116	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	c9e74ec5-89ba-485c-8cd9-95c8ab8779e9	\N	{"id": "c9e74ec5-89ba-485c-8cd9-95c8ab8779e9", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "b34aa434-7514-4794-873a-65bed73e8a6d", "created_at": "2026-09-02T04:38:54.833462+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:54.833462+00
123	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	floor	9502e6d7-da72-474e-b5c3-9e05f942b0ea	\N	{"id": "9502e6d7-da72-474e-b5c3-9e05f942b0ea", "name": "Floor 6", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:58.009857+00:00", "deleted_at": null, "sort_order": 6}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:58.009857+00
126	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	c8aaaad4-5239-4a2f-9c29-ac777a70018e	\N	{"id": "c8aaaad4-5239-4a2f-9c29-ac777a70018e", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "356c128a-768d-4b68-9302-415386c1a368", "created_at": "2026-09-02T04:38:59.54639+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:59.54639+00
129	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	a784861a-d7b1-4954-8888-46f39f4577e8	\N	{"id": "a784861a-d7b1-4954-8888-46f39f4577e8", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "b89153b8-a8f6-4e57-97db-9cfe023d084c", "created_at": "2026-09-02T04:39:01.080129+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:39:01.080129+00
136	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	payment	payment	8f011e55-c5cf-471f-88b1-fe2a0c448ee7	\N	\N	56061b17-ddef-4daa-8583-0dd33a9ccfe7	2026-09-02 05:00:57.050933+00
117	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	7460b71f-a895-45ff-8125-73375483b798	\N	{"id": "7460b71f-a895-45ff-8125-73375483b798", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "b34aa434-7514-4794-873a-65bed73e8a6d", "created_at": "2026-09-02T04:38:55.241395+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:55.241395+00
120	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	403594ba-8a44-4fa7-878d-b62ef41d7b3a	\N	{"id": "403594ba-8a44-4fa7-878d-b62ef41d7b3a", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "f44dd6b7-0659-430d-8acd-88bd62ca683b", "created_at": "2026-09-02T04:38:56.779856+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:56.779856+00
124	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	356c128a-768d-4b68-9302-415386c1a368	\N	{"id": "356c128a-768d-4b68-9302-415386c1a368", "number": "601", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "9502e6d7-da72-474e-b5c3-9e05f942b0ea", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:58.425766+00:00", "deleted_at": null, "cooling_type": "non_ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:58.425766+00
127	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	8bc79c60-edfe-460a-93fc-33fea977f2e8	\N	{"id": "8bc79c60-edfe-460a-93fc-33fea977f2e8", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "356c128a-768d-4b68-9302-415386c1a368", "created_at": "2026-09-02T04:38:59.954766+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:59.954766+00
130	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	bd319951-f8a1-45ef-bcec-b49c984a29cc	\N	{"id": "bd319951-f8a1-45ef-bcec-b49c984a29cc", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "b89153b8-a8f6-4e57-97db-9cfe023d084c", "created_at": "2026-09-02T04:39:01.490954+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:39:01.490954+00
133	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	cce2b864-7f07-4fc3-be48-8596eedd3b0c	\N	{"id": "cce2b864-7f07-4fc3-be48-8596eedd3b0c", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "69cbcdcb-a021-4a89-905a-a48ecab210ab", "created_at": "2026-09-02T04:39:03.032697+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:39:03.032697+00
118	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	5bdd522b-c4f3-46c2-84b9-e35558456fc2	\N	{"id": "5bdd522b-c4f3-46c2-84b9-e35558456fc2", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "b34aa434-7514-4794-873a-65bed73e8a6d", "created_at": "2026-09-02T04:38:55.649928+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:55.649928+00
121	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	05cc22fd-ca22-4b98-8f98-985408455a60	\N	{"id": "05cc22fd-ca22-4b98-8f98-985408455a60", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "f44dd6b7-0659-430d-8acd-88bd62ca683b", "created_at": "2026-09-02T04:38:57.19805+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:57.19805+00
128	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	b89153b8-a8f6-4e57-97db-9cfe023d084c	\N	{"id": "b89153b8-a8f6-4e57-97db-9cfe023d084c", "number": "602", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "9502e6d7-da72-474e-b5c3-9e05f942b0ea", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:39:00.362565+00:00", "deleted_at": null, "cooling_type": "non_ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:39:00.362565+00
131	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	d954ac81-f9e7-4710-a472-2aa571044c49	\N	{"id": "d954ac81-f9e7-4710-a472-2aa571044c49", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "b89153b8-a8f6-4e57-97db-9cfe023d084c", "created_at": "2026-09-02T04:39:01.900322+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:39:01.900322+00
134	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	5f937efc-1c71-4c70-a63c-d9628dceb37d	\N	{"id": "5f937efc-1c71-4c70-a63c-d9628dceb37d", "label": "B", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "69cbcdcb-a021-4a89-905a-a48ecab210ab", "created_at": "2026-09-02T04:39:03.450623+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:39:03.450623+00
119	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	f44dd6b7-0659-430d-8acd-88bd62ca683b	\N	{"id": "f44dd6b7-0659-430d-8acd-88bd62ca683b", "number": "503", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "88f80763-7533-4291-94d5-a4ac46adbe3b", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:38:56.065268+00:00", "deleted_at": null, "cooling_type": "non_ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:56.065268+00
122	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	8d955a62-0851-4f2c-a839-a98795659a8c	\N	{"id": "8d955a62-0851-4f2c-a839-a98795659a8c", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "f44dd6b7-0659-430d-8acd-88bd62ca683b", "created_at": "2026-09-02T04:38:57.600711+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:57.600711+00
125	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	dcc15f52-33f0-4acd-818d-d3b971d9a536	\N	{"id": "dcc15f52-33f0-4acd-818d-d3b971d9a536", "label": "A", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "356c128a-768d-4b68-9302-415386c1a368", "created_at": "2026-09-02T04:38:59.138557+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:38:59.138557+00
132	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	room	69cbcdcb-a021-4a89-905a-a48ecab210ab	\N	{"id": "69cbcdcb-a021-4a89-905a-a48ecab210ab", "number": "603", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "floor_id": "9502e6d7-da72-474e-b5c3-9e05f942b0ea", "branch_id": "72a49ed9-6a5f-4378-b601-a5f309c1d1e3", "created_at": "2026-09-02T04:39:02.308884+00:00", "deleted_at": null, "cooling_type": "non_ac"}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:39:02.308884+00
135	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	bed	0c7f2426-7d93-4c08-b12a-18d1d299bedf	\N	{"id": "0c7f2426-7d93-4c08-b12a-18d1d299bedf", "label": "C", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "room_id": "69cbcdcb-a021-4a89-905a-a48ecab210ab", "created_at": "2026-09-02T04:39:03.84676+00:00", "deleted_at": null}	7917b7d8-9657-42fd-a7b8-b7c81100388c	2026-09-02 04:39:03.84676+00
137	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	floor	57bb145c-0b47-4aa0-8b68-755267f4babb	\N	{"id": "57bb145c-0b47-4aa0-8b68-755267f4babb", "name": "Floor 3", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "created_at": "2026-09-02T06:50:19.750035+00:00", "deleted_at": null, "sort_order": 3}	6182ad94-a986-46f8-8dfe-d11106de53ef	2026-09-02 06:50:19.750035+00
138	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	floor	ef62d558-62cd-450e-97ce-6999e0c6347f	\N	{"id": "ef62d558-62cd-450e-97ce-6999e0c6347f", "name": "Floor 8", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "created_at": "2026-09-02T08:02:34.924776+00:00", "deleted_at": null, "sort_order": 0}	8e940caf-58d6-4673-a92e-ab4c59799918	2026-09-02 08:02:34.924776+00
139	cb68eae7-2955-5cae-e440-0a0190731fe1	1fd63177-2f43-4e85-997c-03314955bfca	create	floor	86d5f4ad-be3d-4ab5-b300-4b0a3f3b75fc	\N	{"id": "86d5f4ad-be3d-4ab5-b300-4b0a3f3b75fc", "name": "Floor 4", "org_id": "cb68eae7-2955-5cae-e440-0a0190731fe1", "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "created_at": "2026-09-02T08:33:20.141691+00:00", "deleted_at": null, "sort_order": 4}	a7b18acd-3c66-43fa-9732-e2a730c68020	2026-09-02 08:33:20.141691+00
\.


--
-- Data for Name: away_periods; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.away_periods (id, org_id, resident_id, from_date, to_date, note, status, created_at, cancelled_at, deleted_at) FROM stdin;
e36215ea-1a98-5edb-ffb5-5e87260a6b38	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	2026-07-10	2026-07-14	Office offsite in Goa	active	2026-07-05 07:30:00+00	\N	\N
fed38738-734c-4eb0-d929-bafad7d9a037	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	2026-06-20	2026-06-22	Weekend trip to Mysuru	cancelled	2026-06-15 14:00:00+00	2026-06-18 03:15:00+00	\N
5bbe78c4-e741-9301-0bb0-e7abe8705b9a	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	2026-08-20	2026-08-25	Travelling home for Onam	cancelled	2026-08-09 14:45:00+00	2026-08-15 22:41:40.702433+00	\N
\.


--
-- Data for Name: beds; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.beds (id, org_id, room_id, label, created_at, deleted_at) FROM stdin;
9ba09c1f-318d-41e1-9c27-f5e4ed463dc5	32f33aef-dc6a-4caf-9f05-794f65f30c8a	c877986d-c1a3-4591-b584-5b6e7268ef77	A	2026-08-11 08:33:20.972622+00	\N
3ce67705-76b3-4450-8c12-7f0e26a7c424	32f33aef-dc6a-4caf-9f05-794f65f30c8a	c877986d-c1a3-4591-b584-5b6e7268ef77	B	2026-08-11 08:33:20.972622+00	\N
41216d53-75b8-49a2-8b16-cb3882cd64c7	32f33aef-dc6a-4caf-9f05-794f65f30c8a	361dea63-d579-4f90-b021-777558e8a38c	A	2026-08-11 08:33:20.972622+00	\N
348fc045-19aa-481d-be5a-e68e0e9632e8	32f33aef-dc6a-4caf-9f05-794f65f30c8a	361dea63-d579-4f90-b021-777558e8a38c	B	2026-08-11 08:33:20.972622+00	\N
2bff133d-5233-4d30-8810-6784ce020055	32f33aef-dc6a-4caf-9f05-794f65f30c8a	361dea63-d579-4f90-b021-777558e8a38c	C	2026-08-11 08:33:20.972622+00	\N
bb562408-b2d8-4019-ad2c-fe4032770a3a	32f33aef-dc6a-4caf-9f05-794f65f30c8a	6c580138-d35d-40c8-9f6e-94af09254f27	A	2026-08-11 08:33:20.972622+00	\N
d85994fb-fa05-48bf-833a-fe810c939ec2	32f33aef-dc6a-4caf-9f05-794f65f30c8a	6c580138-d35d-40c8-9f6e-94af09254f27	B	2026-08-11 08:33:20.972622+00	\N
e791c293-3068-4e3e-b151-1cbdfbd57910	32f33aef-dc6a-4caf-9f05-794f65f30c8a	6c580138-d35d-40c8-9f6e-94af09254f27	C	2026-08-11 08:33:20.972622+00	\N
11c7e1f5-9a5d-4e4b-baf2-51a7dbd59886	32f33aef-dc6a-4caf-9f05-794f65f30c8a	6c580138-d35d-40c8-9f6e-94af09254f27	D	2026-08-11 08:33:20.972622+00	\N
5a3c2678-6dff-4ff1-bfb8-de5d69f478c5	32f33aef-dc6a-4caf-9f05-794f65f30c8a	12c562a7-5472-4fb2-8a8a-7f3113c05da0	A	2026-08-11 08:33:20.972622+00	\N
1872adda-9aaf-4d6a-9c13-39fa35e19df0	32f33aef-dc6a-4caf-9f05-794f65f30c8a	12c562a7-5472-4fb2-8a8a-7f3113c05da0	B	2026-08-11 08:33:20.972622+00	\N
1aa3b446-a536-4717-8aeb-00933aa27868	32f33aef-dc6a-4caf-9f05-794f65f30c8a	f1fc9814-e0f6-4797-844c-511b839f3660	A	2026-08-11 08:33:20.972622+00	\N
be5451c7-7af2-43a9-af70-d6f18bb31768	32f33aef-dc6a-4caf-9f05-794f65f30c8a	f1fc9814-e0f6-4797-844c-511b839f3660	B	2026-08-11 08:33:20.972622+00	\N
49e0fbcd-fa09-4296-b103-bfb4d106c48b	32f33aef-dc6a-4caf-9f05-794f65f30c8a	f1fc9814-e0f6-4797-844c-511b839f3660	C	2026-08-11 08:33:20.972622+00	\N
af9dd177-134f-401c-b043-8ca067203d47	32f33aef-dc6a-4caf-9f05-794f65f30c8a	23dedd21-72b1-4b0b-b758-1129da3833ee	A	2026-08-11 08:33:20.972622+00	\N
762b5ab1-ba32-4f9e-8b66-cfdfe02676d1	32f33aef-dc6a-4caf-9f05-794f65f30c8a	23dedd21-72b1-4b0b-b758-1129da3833ee	B	2026-08-11 08:33:20.972622+00	\N
5af05054-1fee-4278-bed9-3880e6353fcd	32f33aef-dc6a-4caf-9f05-794f65f30c8a	23dedd21-72b1-4b0b-b758-1129da3833ee	C	2026-08-11 08:33:20.972622+00	\N
5c292922-53a1-442b-b481-2df87662aa44	32f33aef-dc6a-4caf-9f05-794f65f30c8a	23dedd21-72b1-4b0b-b758-1129da3833ee	D	2026-08-11 08:33:20.972622+00	\N
422d508e-758e-42a4-9dbd-b95d5b78d5ef	32f33aef-dc6a-4caf-9f05-794f65f30c8a	173e448b-004f-469e-9762-3a2a200eb5f5	A	2026-08-11 08:33:20.972622+00	\N
c86e715b-9dca-4f14-8092-af1a4b03019f	32f33aef-dc6a-4caf-9f05-794f65f30c8a	173e448b-004f-469e-9762-3a2a200eb5f5	B	2026-08-11 08:33:20.972622+00	\N
fbac3c14-cb78-4324-b3dd-8d88e8faea39	32f33aef-dc6a-4caf-9f05-794f65f30c8a	1ef0757d-14f1-478d-b906-1f618e84564f	A	2026-08-11 08:33:20.972622+00	\N
e492693f-4541-4099-bdca-d5fe53cbf79f	32f33aef-dc6a-4caf-9f05-794f65f30c8a	1ef0757d-14f1-478d-b906-1f618e84564f	B	2026-08-11 08:33:20.972622+00	\N
e464bdea-1d22-4ffc-9b49-fe084362628e	32f33aef-dc6a-4caf-9f05-794f65f30c8a	1ef0757d-14f1-478d-b906-1f618e84564f	C	2026-08-11 08:33:20.972622+00	\N
98138d35-7c2c-409f-b837-d51bee74db2d	32f33aef-dc6a-4caf-9f05-794f65f30c8a	a10297a6-cac6-40e6-84f3-e2e73bcd3cdc	A	2026-08-11 08:33:20.972622+00	\N
bae9f5be-e78c-4748-9ca3-d8a62a7462b5	32f33aef-dc6a-4caf-9f05-794f65f30c8a	a10297a6-cac6-40e6-84f3-e2e73bcd3cdc	B	2026-08-11 08:33:20.972622+00	\N
660cf8c1-262f-4146-9073-fda5b34a232c	32f33aef-dc6a-4caf-9f05-794f65f30c8a	a10297a6-cac6-40e6-84f3-e2e73bcd3cdc	C	2026-08-11 08:33:20.972622+00	\N
e3b45c94-b410-49c6-9660-85f0da58943c	32f33aef-dc6a-4caf-9f05-794f65f30c8a	a10297a6-cac6-40e6-84f3-e2e73bcd3cdc	D	2026-08-11 08:33:20.972622+00	\N
2c7318f0-b1df-4b26-ac17-966daa22f4d5	cb68eae7-2955-5cae-e440-0a0190731fe1	16f36aa2-c81f-46a4-9079-2c38b760eadb	A	2026-09-02 06:50:19.750035+00	\N
50fcb878-d7f9-4e58-a853-d5224c2d2e12	cb68eae7-2955-5cae-e440-0a0190731fe1	16f36aa2-c81f-46a4-9079-2c38b760eadb	B	2026-09-02 06:50:19.750035+00	\N
fe89ca51-30e0-499d-8812-fa3f4ee3faa4	cb68eae7-2955-5cae-e440-0a0190731fe1	16f36aa2-c81f-46a4-9079-2c38b760eadb	C	2026-09-02 06:50:19.750035+00	\N
cf56437f-597f-4ced-b3c5-86571ad6e5ba	cb68eae7-2955-5cae-e440-0a0190731fe1	e270d6af-009e-4d9a-94c9-243721c4c51b	A	2026-09-02 06:50:19.750035+00	\N
572495e9-2b61-489c-9967-0e38e604583a	cb68eae7-2955-5cae-e440-0a0190731fe1	e270d6af-009e-4d9a-94c9-243721c4c51b	B	2026-09-02 06:50:19.750035+00	\N
70010657-85cc-efaa-c365-3d5c22a4a8fe	cb68eae7-2955-5cae-e440-0a0190731fe1	e3beab34-30cc-d1b5-b04d-2f85fdc035ba	A	2026-01-05 04:15:00+00	\N
7948c724-ca8b-f399-6078-93417941426f	cb68eae7-2955-5cae-e440-0a0190731fe1	e3beab34-30cc-d1b5-b04d-2f85fdc035ba	B	2026-01-05 04:15:00+00	\N
0dee25ab-d862-718a-32aa-7f8ed96017df	cb68eae7-2955-5cae-e440-0a0190731fe1	e3beab34-30cc-d1b5-b04d-2f85fdc035ba	C	2026-01-05 04:15:00+00	\N
ed724e26-59f9-9647-6a98-f39f40be3e2a	cb68eae7-2955-5cae-e440-0a0190731fe1	e918a2e5-f700-12d1-5cd4-28f8a5340024	A	2026-01-05 04:15:00+00	\N
1bc87e0d-6590-4946-c659-fbd917e6edb0	cb68eae7-2955-5cae-e440-0a0190731fe1	e918a2e5-f700-12d1-5cd4-28f8a5340024	B	2026-01-05 04:15:00+00	\N
7b2afc6d-670d-58a4-e0e8-8e50eb97c400	cb68eae7-2955-5cae-e440-0a0190731fe1	6b0c7dc0-8899-a1c8-cbed-9b4667ac33ea	A	2026-01-05 04:15:00+00	\N
5d84c481-23f6-bcb6-9a01-039c69a36c53	cb68eae7-2955-5cae-e440-0a0190731fe1	6b0c7dc0-8899-a1c8-cbed-9b4667ac33ea	B	2026-01-05 04:15:00+00	\N
3e62dc53-23cc-4084-263a-0f64de095253	cb68eae7-2955-5cae-e440-0a0190731fe1	6b0c7dc0-8899-a1c8-cbed-9b4667ac33ea	C	2026-01-05 04:15:00+00	\N
d21cb933-e9f8-d653-15dd-13951704085e	cb68eae7-2955-5cae-e440-0a0190731fe1	6bf8fab9-d00e-e997-3883-cdc0c68e655a	A	2026-01-05 04:15:00+00	\N
3a54dcdb-cc4f-ba2b-53be-e431d46b214d	cb68eae7-2955-5cae-e440-0a0190731fe1	6bf8fab9-d00e-e997-3883-cdc0c68e655a	B	2026-01-05 04:15:00+00	\N
b23584b1-aa29-2ac7-d762-4bd5a54d4c6b	cb68eae7-2955-5cae-e440-0a0190731fe1	d8b3aaea-ef8b-063b-a27a-9f14ff96bb13	A	2026-01-05 04:15:00+00	\N
9abcdb1c-0bf4-10c0-1632-298a8f001a58	cb68eae7-2955-5cae-e440-0a0190731fe1	d8b3aaea-ef8b-063b-a27a-9f14ff96bb13	B	2026-01-05 04:15:00+00	\N
b8f2521a-2c87-3d26-2634-679cf3641931	cb68eae7-2955-5cae-e440-0a0190731fe1	d8b3aaea-ef8b-063b-a27a-9f14ff96bb13	C	2026-01-05 04:15:00+00	\N
13b1f758-96ae-2e72-ff1c-751f07da613d	cb68eae7-2955-5cae-e440-0a0190731fe1	61f0db3a-3817-98dc-48a4-d87f0db9ec5e	A	2026-01-05 04:15:00+00	\N
109fbfb0-372e-9065-fc18-78755f87e3df	cb68eae7-2955-5cae-e440-0a0190731fe1	61f0db3a-3817-98dc-48a4-d87f0db9ec5e	B	2026-01-05 04:15:00+00	\N
92b80a1c-203c-4f2e-8739-95cf875cbad3	cb68eae7-2955-5cae-e440-0a0190731fe1	36a45727-4adc-4fb9-a2a7-7996f67ed74a	A	2026-09-02 08:33:20.141691+00	\N
6f0f2203-a052-406f-9374-8cc60c6877e0	cb68eae7-2955-5cae-e440-0a0190731fe1	36a45727-4adc-4fb9-a2a7-7996f67ed74a	B	2026-09-02 08:33:20.141691+00	\N
b0fe13a2-6993-4689-9cd7-db053bedec3a	cb68eae7-2955-5cae-e440-0a0190731fe1	36a45727-4adc-4fb9-a2a7-7996f67ed74a	C	2026-09-02 08:33:20.141691+00	\N
55937992-71dd-4ce6-ae3e-62fc4d3d494c	cb68eae7-2955-5cae-e440-0a0190731fe1	993ce46e-f8ae-4a18-b835-984890c7b629	A	2026-09-02 08:33:20.141691+00	\N
\.


--
-- Data for Name: branches; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.branches (id, org_id, name, address, created_at, deleted_at) FROM stdin;
af1202c3-9efe-4de3-a5ee-ce7383b81e0b	32f33aef-dc6a-4caf-9f05-794f65f30c8a	Main Branch	\N	2026-08-11 07:57:24.539473+00	\N
bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	cb68eae7-2955-5cae-e440-0a0190731fe1	HSR Layout	#42, 27th Main Road, HSR Layout Sector 2, Bengaluru, Karnataka 560102	2026-01-05 04:05:00+00	\N
f67789c7-edd7-4494-a6fa-3b6fc82e5f28	5d49310f-864e-4a65-926b-f4168d1dd9b6	Main Branch	\N	2026-09-01 06:21:54.990901+00	\N
\.


--
-- Data for Name: change_requests; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.change_requests (id, org_id, entity_type, entity_id, operation, proposed, requested_by, status, created_at, decided_at, decided_by) FROM stdin;
d2a49116-50cc-3c54-95c5-4da0f31050c3	cb68eae7-2955-5cae-e440-0a0190731fe1	resident	b90bbc07-df8e-3bed-1d7d-7c245e23912a	update	{"rent_paise": 1300000}	0f2c58a3-c922-48ef-9571-68c5eb743a87	pending	2026-08-11 07:00:00+00	\N	\N
6396d949-d4cc-8ea4-d658-417543c59d12	cb68eae7-2955-5cae-e440-0a0190731fe1	room	6bf8fab9-d00e-e997-3883-cdc0c68e655a	update	{"cooling_type": "non_ac"}	0f2c58a3-c922-48ef-9571-68c5eb743a87	pending	2026-08-12 03:45:00+00	\N	\N
fbc91afd-ee0a-e933-9280-bcea37913810	cb68eae7-2955-5cae-e440-0a0190731fe1	resident	b90bbc07-df8e-3bed-1d7d-7c245e23912a	update	{"deposit_paise": 0}	0f2c58a3-c922-48ef-9571-68c5eb743a87	rejected	2026-08-05 09:10:00+00	2026-08-06 04:35:00+00	1fd63177-2f43-4e85-997c-03314955bfca
f35b09c3-58a4-4166-a395-2bd8e02c5dea	cb68eae7-2955-5cae-e440-0a0190731fe1	floor	b7b5e3ff-175b-42d9-a727-cab68c62b6b4	create	{"name": "Floor 4", "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "sort_order": 4}	0f2c58a3-c922-48ef-9571-68c5eb743a87	approved	2026-09-02 04:34:35.600005+00	2026-09-02 04:34:54.832393+00	1fd63177-2f43-4e85-997c-03314955bfca
a7f6222a-a4d5-423e-a0f0-f9e19e2ce945	cb68eae7-2955-5cae-e440-0a0190731fe1	room	298afa66-6b9f-4fdc-946f-a0c86e079052	create	{"number": "301", "floor_id": null, "branch_id": "bdbfddf5-eb26-9e23-5fd4-2ba1307460a4", "cooling_type": "ac"}	0f2c58a3-c922-48ef-9571-68c5eb743a87	approved	2026-09-02 04:34:36.272469+00	2026-09-02 04:34:55.782246+00	1fd63177-2f43-4e85-997c-03314955bfca
c07a99ba-c4c9-4895-b75c-527b870ae712	cb68eae7-2955-5cae-e440-0a0190731fe1	floor	1114b103-e9e5-4ac6-af74-e02c5b62016c	delete	null	0f2c58a3-c922-48ef-9571-68c5eb743a87	pending	2026-09-02 04:35:17.900059+00	\N	\N
def6fa03-d324-4c8f-bf41-a866178aec0e	cb68eae7-2955-5cae-e440-0a0190731fe1	room	6b0c7dc0-8899-a1c8-cbed-9b4667ac33ea	update	{"floor_id": "1114b103-e9e5-4ac6-af74-e02c5b62016c"}	0f2c58a3-c922-48ef-9571-68c5eb743a87	pending	2026-09-02 05:04:31.99268+00	\N	\N
\.


--
-- Data for Name: complaint_events; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.complaint_events (id, org_id, complaint_id, from_state, to_state, by_user, created_at) FROM stdin;
11	cb68eae7-2955-5cae-e440-0a0190731fe1	004972ae-d2dc-fd52-59c9-601da65a83fc	\N	open	92b0cc9d-1633-cbde-3356-6c31054360b9	2026-08-02 02:15:00+00
12	cb68eae7-2955-5cae-e440-0a0190731fe1	004972ae-d2dc-fd52-59c9-601da65a83fc	open	assigned	0f2c58a3-c922-48ef-9571-68c5eb743a87	2026-08-02 03:40:00+00
13	cb68eae7-2955-5cae-e440-0a0190731fe1	004972ae-d2dc-fd52-59c9-601da65a83fc	assigned	in_progress	0f2c58a3-c922-48ef-9571-68c5eb743a87	2026-08-04 05:50:00+00
14	cb68eae7-2955-5cae-e440-0a0190731fe1	b20fc105-a806-ef04-be73-e107cce6729a	\N	open	92b0cc9d-1633-cbde-3356-6c31054360b9	2026-07-18 16:00:00+00
15	cb68eae7-2955-5cae-e440-0a0190731fe1	b20fc105-a806-ef04-be73-e107cce6729a	open	assigned	0f2c58a3-c922-48ef-9571-68c5eb743a87	2026-07-19 03:30:00+00
16	cb68eae7-2955-5cae-e440-0a0190731fe1	b20fc105-a806-ef04-be73-e107cce6729a	assigned	in_progress	0f2c58a3-c922-48ef-9571-68c5eb743a87	2026-07-20 08:55:00+00
17	cb68eae7-2955-5cae-e440-0a0190731fe1	b20fc105-a806-ef04-be73-e107cce6729a	in_progress	resolved	0f2c58a3-c922-48ef-9571-68c5eb743a87	2026-07-22 09:40:00+00
18	cb68eae7-2955-5cae-e440-0a0190731fe1	ee59c4e8-12cd-74f0-e976-ef97e65553f0	\N	open	0f2c58a3-c922-48ef-9571-68c5eb743a87	2026-08-10 04:35:00+00
\.


--
-- Data for Name: complaints; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.complaints (id, org_id, branch_id, raised_by, title, description, state, is_business_level, created_at, updated_at, deleted_at) FROM stdin;
004972ae-d2dc-fd52-59c9-601da65a83fc	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	92b0cc9d-1633-cbde-3356-6c31054360b9	Geyser in 102 bathroom not heating	The geyser trips within a minute of switching on. Water stays cold in the mornings.	in_progress	f	2026-08-02 02:15:00+00	2026-08-04 05:50:00+00	\N
b20fc105-a806-ef04-be73-e107cce6729a	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	92b0cc9d-1633-cbde-3356-6c31054360b9	Wi-Fi drops between 9pm and 11pm	Connection on the second floor drops repeatedly during peak hours. Speed test shows under 1 Mbps.	resolved	f	2026-07-18 16:00:00+00	2026-07-22 09:40:00+00	\N
ee59c4e8-12cd-74f0-e976-ef97e65553f0	cb68eae7-2955-5cae-e440-0a0190731fe1	\N	0f2c58a3-c922-48ef-9571-68c5eb743a87	Water tanker vendor overcharging since July	The tanker vendor raised the per-trip rate twice in one month without notice. Needs an owner decision on switching vendors.	open	t	2026-08-10 04:35:00+00	2026-08-10 04:35:00+00	\N
\.


--
-- Data for Name: floors; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.floors (id, org_id, branch_id, name, sort_order, created_at, deleted_at) FROM stdin;
1114b103-e9e5-4ac6-af74-e02c5b62016c	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	Floor 1	1	2026-09-01 19:47:42.870435+00	\N
e5c3ce85-2e17-446f-ab2d-4d824b1cefad	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	Floor 2	2	2026-09-01 19:47:43.55756+00	\N
57bb145c-0b47-4aa0-8b68-755267f4babb	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	Floor 3	3	2026-09-02 06:50:19.750035+00	\N
86d5f4ad-be3d-4ab5-b300-4b0a3f3b75fc	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	Floor 4	4	2026-09-02 08:33:20.141691+00	\N
\.


--
-- Data for Name: invoice_sequences; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.invoice_sequences (period, next_number, org_id) FROM stdin;
2608	2	32f33aef-dc6a-4caf-9f05-794f65f30c8a
2602	2	cb68eae7-2955-5cae-e440-0a0190731fe1
2603	2	cb68eae7-2955-5cae-e440-0a0190731fe1
2604	2	cb68eae7-2955-5cae-e440-0a0190731fe1
2605	3	cb68eae7-2955-5cae-e440-0a0190731fe1
2606	2	cb68eae7-2955-5cae-e440-0a0190731fe1
2607	2	cb68eae7-2955-5cae-e440-0a0190731fe1
2608	4	cb68eae7-2955-5cae-e440-0a0190731fe1
2609	10	cb68eae7-2955-5cae-e440-0a0190731fe1
\.


--
-- Data for Name: move_out_settlements; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.move_out_settlements (id, org_id, resident_id, last_day, notice_days, rent_to_date_paise, paid_total_paise, outstanding_paise, deposit_held_paise, deductions_total_paise, refund_paise, forfeited, note, settled_by, settled_at) FROM stdin;
b4d4dcee-1bf9-752a-907e-884a141d5e38	cb68eae7-2955-5cae-e440-0a0190731fe1	f99f56ee-c00d-87b9-3503-a320594f36b8	2026-05-31	30	5440000	4800000	640000	2400000	240000	1520000	f	refund = deposit - outstanding - deductions	1fd63177-2f43-4e85-997c-03314955bfca	2026-06-02 10:30:00+00
\.


--
-- Data for Name: move_out_deductions; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.move_out_deductions (id, settlement_id, reason, amount_paise) FROM stdin;
70349520-166f-8183-6036-acc07cea8251	b4d4dcee-1bf9-752a-907e-884a141d5e38	Wall repainting after nail damage	150000
02a1dc67-042c-6ea0-afea-1ce4cd14bc1d	b4d4dcee-1bf9-752a-907e-884a141d5e38	Unpaid electricity share (Apr-May)	90000
\.


--
-- Data for Name: notices; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.notices (id, org_id, branch_id, title, body, created_by, created_at, deleted_at) FROM stdin;
57c9bb29-e1b5-e2de-e4e1-2e5080bbba0d	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	Rent due on the 5th	A reminder that monthly rent is due by the 5th. UPI and cash are both accepted; collect your receipt from the manager on the same day.	1fd63177-2f43-4e85-997c-03314955bfca	2026-08-01 03:30:00+00	\N
453d040d-1b28-4995-0c2d-973dc9e1e61e	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	Water supply maintenance on 14 Aug	The overhead tank will be cleaned on 14 August between 10am and 2pm. Please store drinking water in advance.	0f2c58a3-c922-48ef-9571-68c5eb743a87	2026-08-08 12:50:00+00	\N
6ae999d2-b0a1-415f-1589-5d2f12b419b0	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	Independence Day special dinner	Special dinner menu on 15 August at 8pm in the common dining area. Let the kitchen know by 13 August if you will be away.	0f2c58a3-c922-48ef-9571-68c5eb743a87	2026-08-11 11:30:00+00	\N
\.


--
-- Data for Name: payments; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.payments (id, org_id, resident_id, amount_paise, paid_at, recorded_by, method, status, upi_ref, reconciled_at, idempotency_key, created_at, deleted_at, kind) FROM stdin;
c2e1aa76-2d75-4d68-9c8e-5d3d5d6f8a01	32f33aef-dc6a-4caf-9f05-794f65f30c8a	1d996da5-a43c-4f1d-989a-818f7837c64a	1500000	2026-08-09 06:12:06+00	\N	cash	recorded	\N	\N	\N	2026-08-12 15:42:50.188443+00	2026-08-12 17:28:45.087524+00	rent
be03aa26-0781-a25f-22f7-6919256107dd	cb68eae7-2955-5cae-e440-0a0190731fe1	f99f56ee-c00d-87b9-3503-a320594f36b8	1200000	2026-02-03 13:50:00+00	1fd63177-2f43-4e85-997c-03314955bfca	cash	recorded	\N	\N	seed-naveen:2602	2026-02-03 13:50:00+00	\N	rent
21366245-4651-04f7-cf5d-e0ca6707c41e	cb68eae7-2955-5cae-e440-0a0190731fe1	f99f56ee-c00d-87b9-3503-a320594f36b8	1200000	2026-04-03 14:40:00+00	0f2c58a3-c922-48ef-9571-68c5eb743a87	cash	recorded	\N	\N	seed-naveen:2604	2026-04-03 14:40:00+00	\N	rent
63b596f9-aee9-e9a4-4cc0-6c0dff331913	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	1250000	2026-07-04 13:45:00+00	0f2c58a3-c922-48ef-9571-68c5eb743a87	cash	recorded	\N	\N	seed-krish:2607	2026-07-04 13:45:00+00	\N	rent
79ec5038-7b4d-4609-ac0b-040aaa3c9c3c	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	500000	2026-08-12 17:39:28.027079+00	1fd63177-2f43-4e85-997c-03314955bfca	cash	recorded	\N	\N	\N	2026-08-12 17:39:28.027079+00	\N	rent
ed7b1f5b-fa89-91e2-fd7d-a7f9180a3270	cb68eae7-2955-5cae-e440-0a0190731fe1	f99f56ee-c00d-87b9-3503-a320594f36b8	1200000	2026-03-04 12:35:00+00	1fd63177-2f43-4e85-997c-03314955bfca	cash	recorded	\N	\N	seed-naveen:2603	2026-03-04 12:35:00+00	\N	rent
b6612dcc-1703-a10e-1aba-a77caa39c66c	cb68eae7-2955-5cae-e440-0a0190731fe1	f99f56ee-c00d-87b9-3503-a320594f36b8	1200000	2026-05-05 12:15:00+00	0f2c58a3-c922-48ef-9571-68c5eb743a87	cash	recorded	\N	\N	seed-naveen:2605	2026-05-05 12:15:00+00	\N	rent
672a55a0-251a-2561-6a09-2a289efc2491	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	1250000	2026-06-05 13:00:00+00	1fd63177-2f43-4e85-997c-03314955bfca	cash	recorded	\N	\N	seed-krish:2606	2026-06-05 13:00:00+00	\N	rent
1b6d5372-a3fc-4b1b-a386-64e93be27886	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	100000	2026-08-12 19:29:19.112453+00	0f2c58a3-c922-48ef-9571-68c5eb743a87	cash	recorded	\N	\N	\N	2026-08-12 19:29:19.112453+00	2026-08-12 19:29:34.035004+00	rent
8800dd1b-09e2-0149-e643-6824e0b0d08f	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	2500000	2026-05-31 11:30:00+00	1fd63177-2f43-4e85-997c-03314955bfca	cash	recorded	\N	\N	seed-krish:deposit	2026-05-31 11:30:00+00	\N	deposit
70899a74-1a00-4d4d-a91a-6601d2f42d77	cb68eae7-2955-5cae-e440-0a0190731fe1	2d83dfa8-338e-44da-bc3c-c864ac397d74	41666	2026-09-01 10:05:32.363975+00	1fd63177-2f43-4e85-997c-03314955bfca	cash	recorded	\N	\N	\N	2026-09-01 10:05:32.363975+00	2026-09-01 10:05:43.744808+00	rent
a1ff2dfe-3b26-4c65-aa38-3634e1ccdefb	cb68eae7-2955-5cae-e440-0a0190731fe1	2d83dfa8-338e-44da-bc3c-c864ac397d74	41666	2026-09-01 10:05:49.84756+00	1fd63177-2f43-4e85-997c-03314955bfca	cash	recorded	\N	\N	\N	2026-09-01 10:05:49.84756+00	\N	rent
39612637-41c6-4aa5-9d2e-8d009d778bef	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	100000	2026-09-02 03:23:40.536377+00	1fd63177-2f43-4e85-997c-03314955bfca	upi	recorded	T2509021450	\N	test-key-A-1788319419	2026-09-02 03:23:40.536377+00	\N	rent
f153e0a1-1423-4924-8314-0176adbccbad	cb68eae7-2955-5cae-e440-0a0190731fe1	2d83dfa8-338e-44da-bc3c-c864ac397d74	50000	2026-09-02 03:23:46.078808+00	1fd63177-2f43-4e85-997c-03314955bfca	cash	recorded	\N	\N	test-key-B-1788319419	2026-09-02 03:23:46.078808+00	\N	rent
f69f3931-15a6-4eca-b2fb-fc2379906d3d	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	100000	2026-09-02 03:27:46.395162+00	1fd63177-2f43-4e85-997c-03314955bfca	upi	recorded	T2509021455	\N	dup-test-1788319664	2026-09-02 03:27:46.395162+00	\N	rent
29ddb942-31de-420a-9a11-ce366f842f1a	cb68eae7-2955-5cae-e440-0a0190731fe1	2d83dfa8-338e-44da-bc3c-c864ac397d74	50000	2026-09-02 03:27:49.882221+00	1fd63177-2f43-4e85-997c-03314955bfca	cash	recorded	\N	\N	cash-1788319668	2026-09-02 03:27:49.882221+00	\N	rent
88a9e699-38d5-4900-8bc1-b2fdf0e6a32a	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	675000	2026-09-02 03:31:36.149547+00	1fd63177-2f43-4e85-997c-03314955bfca	cash	recorded	\N	\N	5c9e7927-bc43-49bd-a26a-0b6d6ff033cd	2026-09-02 03:31:36.149547+00	\N	rent
84533728-d4a5-443c-a720-ac8d8b886a27	cb68eae7-2955-5cae-e440-0a0190731fe1	f99f56ee-c00d-87b9-3503-a320594f36b8	64000	2026-09-02 03:52:53.284166+00	1fd63177-2f43-4e85-997c-03314955bfca	cash	recorded	\N	\N	blue-check-1788321172	2026-09-02 03:52:53.284166+00	\N	rent
8f011e55-c5cf-471f-88b1-fe2a0c448ee7	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	10000	2026-09-02 05:00:57.050933+00	1fd63177-2f43-4e85-997c-03314955bfca	cash	recorded	\N	\N	seq-check-1788325255	2026-09-02 05:00:57.050933+00	\N	rent
\.


--
-- Data for Name: receipts; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.receipts (id, org_id, resident_id, payment_id, receipt_number, amount_paise, storage_key, status, issued_at, deleted_at, invoice_number) FROM stdin;
ae3f0d10-98d6-4d19-8032-252bf36008be	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	39612637-41c6-4aa5-9d2e-8d009d778bef	INV-2609-003	100000	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/b90bbc07-df8e-3bed-1d7d-7c245e23912a/ae3f0d10-98d6-4d19-8032-252bf36008be.pdf	ready	2026-09-02 03:23:40.536377+00	\N	INV-2609-003
e95f75ee-0a1f-4d93-900e-9d1ea4f2b202	32f33aef-dc6a-4caf-9f05-794f65f30c8a	1d996da5-a43c-4f1d-989a-818f7837c64a	c2e1aa76-2d75-4d68-9c8e-5d3d5d6f8a01	INV-2608-001	1500000	receipts/32f33aef-dc6a-4caf-9f05-794f65f30c8a/1d996da5-a43c-4f1d-989a-818f7837c64a/e95f75ee-0a1f-4d93-900e-9d1ea4f2b202.pdf	ready	2026-08-09 06:12:06+00	2026-08-12 17:28:45.087524+00	INV-2608-001
ba7539e9-5b5b-eda3-509f-4be6e358c6bb	cb68eae7-2955-5cae-e440-0a0190731fe1	f99f56ee-c00d-87b9-3503-a320594f36b8	be03aa26-0781-a25f-22f7-6919256107dd	INV-2602-001	1200000	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/f99f56ee-c00d-87b9-3503-a320594f36b8/ba7539e9-5b5b-eda3-509f-4be6e358c6bb.pdf	ready	2026-02-03 13:50:00+00	\N	INV-2602-001
fdc0cfaf-fa75-e5e7-e270-772cf9c3b15d	cb68eae7-2955-5cae-e440-0a0190731fe1	f99f56ee-c00d-87b9-3503-a320594f36b8	ed7b1f5b-fa89-91e2-fd7d-a7f9180a3270	INV-2603-001	1200000	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/f99f56ee-c00d-87b9-3503-a320594f36b8/fdc0cfaf-fa75-e5e7-e270-772cf9c3b15d.pdf	ready	2026-03-04 12:35:00+00	\N	INV-2603-001
243371f1-af80-4a8d-7234-90c17b5e7edb	cb68eae7-2955-5cae-e440-0a0190731fe1	f99f56ee-c00d-87b9-3503-a320594f36b8	21366245-4651-04f7-cf5d-e0ca6707c41e	INV-2604-001	1200000	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/f99f56ee-c00d-87b9-3503-a320594f36b8/243371f1-af80-4a8d-7234-90c17b5e7edb.pdf	ready	2026-04-03 14:40:00+00	\N	INV-2604-001
ba9f620e-ea00-fe2a-23f0-e61de08173fa	cb68eae7-2955-5cae-e440-0a0190731fe1	f99f56ee-c00d-87b9-3503-a320594f36b8	b6612dcc-1703-a10e-1aba-a77caa39c66c	INV-2605-001	1200000	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/f99f56ee-c00d-87b9-3503-a320594f36b8/ba9f620e-ea00-fe2a-23f0-e61de08173fa.pdf	ready	2026-05-05 12:15:00+00	\N	INV-2605-001
b02ecdbe-a9ad-cd61-96db-6ef9f24f62ed	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	672a55a0-251a-2561-6a09-2a289efc2491	INV-2606-001	1250000	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/b90bbc07-df8e-3bed-1d7d-7c245e23912a/b02ecdbe-a9ad-cd61-96db-6ef9f24f62ed.pdf	ready	2026-06-05 13:00:00+00	\N	INV-2606-001
37557a42-77a0-4078-dd96-a77d95cd5438	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	63b596f9-aee9-e9a4-4cc0-6c0dff331913	INV-2607-001	1250000	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/b90bbc07-df8e-3bed-1d7d-7c245e23912a/37557a42-77a0-4078-dd96-a77d95cd5438.pdf	ready	2026-07-04 13:45:00+00	\N	INV-2607-001
e16539cd-e477-443e-9ca4-15c8734ab366	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	79ec5038-7b4d-4609-ac0b-040aaa3c9c3c	INV-2608-002	500000	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/b90bbc07-df8e-3bed-1d7d-7c245e23912a/e16539cd-e477-443e-9ca4-15c8734ab366.pdf	ready	2026-08-12 17:39:28.027079+00	\N	INV-2608-002
00188ce4-6127-4b90-b2d4-67f385f02e10	cb68eae7-2955-5cae-e440-0a0190731fe1	2d83dfa8-338e-44da-bc3c-c864ac397d74	f153e0a1-1423-4924-8314-0176adbccbad	INV-2609-004	50000	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/2d83dfa8-338e-44da-bc3c-c864ac397d74/00188ce4-6127-4b90-b2d4-67f385f02e10.pdf	ready	2026-09-02 03:23:46.078808+00	\N	INV-2609-004
e676b9e3-28df-4c75-8963-ac8f3acb6b45	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	1b6d5372-a3fc-4b1b-a386-64e93be27886	INV-2608-003	100000	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/b90bbc07-df8e-3bed-1d7d-7c245e23912a/e676b9e3-28df-4c75-8963-ac8f3acb6b45.pdf	ready	2026-08-12 19:29:19.112453+00	2026-08-12 19:29:34.035004+00	INV-2608-003
55f5c896-3e1a-2a17-f7af-ad50aedd083b	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	8800dd1b-09e2-0149-e643-6824e0b0d08f	INV-2605-002	2500000	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/b90bbc07-df8e-3bed-1d7d-7c245e23912a/55f5c896-3e1a-2a17-f7af-ad50aedd083b.pdf	ready	2026-05-31 11:30:00+00	\N	INV-2605-002
81c686d2-5f7c-4209-a3d0-89b2fd26043d	cb68eae7-2955-5cae-e440-0a0190731fe1	2d83dfa8-338e-44da-bc3c-c864ac397d74	70899a74-1a00-4d4d-a91a-6601d2f42d77	INV-2609-001	41666	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/2d83dfa8-338e-44da-bc3c-c864ac397d74/81c686d2-5f7c-4209-a3d0-89b2fd26043d.pdf	ready	2026-09-01 10:05:32.363975+00	2026-09-01 10:05:43.744808+00	INV-2609-001
266e09c0-5e43-4341-8a8d-ffe742e22fd3	cb68eae7-2955-5cae-e440-0a0190731fe1	2d83dfa8-338e-44da-bc3c-c864ac397d74	a1ff2dfe-3b26-4c65-aa38-3634e1ccdefb	INV-2609-002	41666	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/2d83dfa8-338e-44da-bc3c-c864ac397d74/266e09c0-5e43-4341-8a8d-ffe742e22fd3.pdf	ready	2026-09-01 10:05:49.84756+00	\N	INV-2609-002
ee1d0e1b-dbff-42fa-8e24-6b36669e612d	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	f69f3931-15a6-4eca-b2fb-fc2379906d3d	INV-2609-005	100000	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/b90bbc07-df8e-3bed-1d7d-7c245e23912a/ee1d0e1b-dbff-42fa-8e24-6b36669e612d.pdf	ready	2026-09-02 03:27:46.395162+00	\N	INV-2609-005
b2efb7d4-7de2-4909-9d96-85741eeb3c3d	cb68eae7-2955-5cae-e440-0a0190731fe1	2d83dfa8-338e-44da-bc3c-c864ac397d74	29ddb942-31de-420a-9a11-ce366f842f1a	INV-2609-006	50000	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/2d83dfa8-338e-44da-bc3c-c864ac397d74/b2efb7d4-7de2-4909-9d96-85741eeb3c3d.pdf	ready	2026-09-02 03:27:49.882221+00	\N	INV-2609-006
89633826-f9bc-4121-9ab9-873985de6906	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	88a9e699-38d5-4900-8bc1-b2fdf0e6a32a	INV-2609-007	675000	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/b90bbc07-df8e-3bed-1d7d-7c245e23912a/89633826-f9bc-4121-9ab9-873985de6906.pdf	ready	2026-09-02 03:31:36.149547+00	\N	INV-2609-007
d2469e7c-609a-4d63-ab18-84b1ff9b5163	cb68eae7-2955-5cae-e440-0a0190731fe1	f99f56ee-c00d-87b9-3503-a320594f36b8	84533728-d4a5-443c-a720-ac8d8b886a27	INV-2609-008	64000	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/f99f56ee-c00d-87b9-3503-a320594f36b8/d2469e7c-609a-4d63-ab18-84b1ff9b5163.pdf	ready	2026-09-02 03:52:53.284166+00	\N	INV-2609-008
0ea05407-0f3b-481f-be0c-f6e99481f4ab	cb68eae7-2955-5cae-e440-0a0190731fe1	b90bbc07-df8e-3bed-1d7d-7c245e23912a	8f011e55-c5cf-471f-88b1-fe2a0c448ee7	INV-2609-009	10000	receipts/cb68eae7-2955-5cae-e440-0a0190731fe1/b90bbc07-df8e-3bed-1d7d-7c245e23912a/0ea05407-0f3b-481f-be0c-f6e99481f4ab.pdf	ready	2026-09-02 05:00:57.050933+00	\N	INV-2609-009
\.


--
-- Data for Name: organizations; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.organizations (id, name, created_at, deleted_at) FROM stdin;
32f33aef-dc6a-4caf-9f05-794f65f30c8a	Supabase Development PG	2026-08-11 07:57:22.986182+00	\N
00000000-0000-0000-0000-000000000001	Hamsa Vilas Platform	2026-08-12 09:22:54.820454+00	\N
cb68eae7-2955-5cae-e440-0a0190731fe1	Hamsa Vilas PG	2026-01-05 04:00:00+00	\N
5d49310f-864e-4a65-926b-f4168d1dd9b6	Hamsa Vilas Dev	2026-09-01 06:21:52.556512+00	\N
\.


--
-- Data for Name: platform_users; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.platform_users (id, org_id, phone, full_name, verified, status, google_sub, created_at, deleted_at, email, email_verified_at) FROM stdin;
00e1e1f4-96d0-43ec-b27e-e5cc04297ffa	32f33aef-dc6a-4caf-9f05-794f65f30c8a	9000012345	Supabase Test Owner	f	active	\N	2026-08-11 07:57:23.886333+00	\N	\N	\N
0f2c58a3-c922-48ef-9571-68c5eb743a87	00000000-0000-0000-0000-000000000001	9000010102	Bhargav	t	active	\N	2026-08-12 16:51:12.207028+00	\N	bhargavraj911@gmail.com	2026-08-12 17:28:45.087524+00
1fd63177-2f43-4e85-997c-03314955bfca	00000000-0000-0000-0000-000000000001	9000010101	Bhargavraj	t	active	\N	2026-08-12 09:31:29.540811+00	\N	owner@aalaya.in	2026-08-12 09:40:30.059218+00
c63ec3fb-6d34-4755-803f-7c078e20abef	00000000-0000-0000-0000-000000000001	\N	\N	f	active	\N	2026-08-12 09:23:49.384712+00	\N	otp.test.20260812@example.com	2026-08-31 16:27:09.062823+00
3d597748-da72-422a-b29d-3d104dd1ec9a	5d49310f-864e-4a65-926b-f4168d1dd9b6	\N	Dev Owner	f	active	\N	2026-09-01 06:21:53.499544+00	\N	owner@hamsavilas.test	\N
0da807f6-32f9-42a3-b164-af11c7584e00	00000000-0000-0000-0000-000000000001	\N	\N	f	active	\N	2026-09-01 06:45:12.887337+00	\N	probe@example.com	\N
c956cebe-792a-4a40-81f4-e058c737d279	00000000-0000-0000-0000-000000000001	\N	\N	f	active	\N	2026-09-01 07:50:14.660875+00	\N	probe3@example.com	\N
d6e71466-8267-4a5c-86e1-4f940ee4c510	00000000-0000-0000-0000-000000000001	\N	\N	f	active	\N	2026-09-01 11:28:31.703787+00	\N	test@test.com	\N
3e91a99c-7b18-48ae-a467-af9c8112acaf	00000000-0000-0000-0000-000000000001	\N	\N	f	active	\N	2026-09-01 16:12:15.404764+00	\N	kishore.bejugama@gmail.com	2026-09-01 16:13:25.655765+00
a3b13fff-6e12-48d5-b5d1-49c8ed34c7d0	00000000-0000-0000-0000-000000000001	\N	\N	f	active	\N	2026-09-01 16:38:03.423317+00	\N	test.owner@aalaya.in	\N
cd97043e-49fe-48d6-bdfc-7ca1a0cb7964	00000000-0000-0000-0000-000000000001	\N	\N	f	active	\N	2026-09-01 17:39:09.300097+00	\N	tenant@hamsavilas.test	\N
92b0cc9d-1633-cbde-3356-6c31054360b9	00000000-0000-0000-0000-000000000001	9000010103	Krish	t	active	\N	2026-05-20 13:05:00+00	\N	kriahkunar2@gmail.com	2026-05-20 13:10:00+00
1a7a2644-5f85-4057-b00b-445646f4f2a4	00000000-0000-0000-0000-000000000001	\N	\N	f	active	\N	2026-09-01 19:27:59.460564+00	\N	explorer.test.0902@example.com	2026-09-01 19:28:36.962889+00
638d9648-01dc-4225-a46d-11b804cae77a	00000000-0000-0000-0000-000000000001	\N	Ashrith Bade	f	active	\N	2026-08-31 14:41:23.628755+00	\N	ashrithbade888@gmail.com	2026-08-31 14:41:27.112558+00
\.


--
-- Data for Name: user_org_roles; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.user_org_roles (user_id, org_id, role, branch_id, membership_status, activated_at, ended_at, id) FROM stdin;
00e1e1f4-96d0-43ec-b27e-e5cc04297ffa	32f33aef-dc6a-4caf-9f05-794f65f30c8a	owner	\N	active	\N	\N	4c3a0f77-e146-422a-9ea8-e73c2d58841e
c63ec3fb-6d34-4755-803f-7c078e20abef	00000000-0000-0000-0000-000000000001	prospect	\N	active	\N	\N	66d55d60-53e1-4d12-9a8c-933bdf7e219d
1fd63177-2f43-4e85-997c-03314955bfca	00000000-0000-0000-0000-000000000001	prospect	\N	active	\N	\N	3f7b03d8-6513-464b-92aa-dc397b24f409
0f2c58a3-c922-48ef-9571-68c5eb743a87	00000000-0000-0000-0000-000000000001	prospect	\N	active	\N	\N	87f03c26-4974-4ef8-bc7f-e98c56c9a33e
1fd63177-2f43-4e85-997c-03314955bfca	32f33aef-dc6a-4caf-9f05-794f65f30c8a	tenant	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	moved_out	\N	2026-08-12 17:28:45.087524+00	b2ad6579-9abd-4694-b9f0-6959590c6de3
1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	owner	\N	active	2026-01-05 04:00:00+00	\N	7aae22e2-194d-6d2b-7320-a76f6158a104
0f2c58a3-c922-48ef-9571-68c5eb743a87	cb68eae7-2955-5cae-e440-0a0190731fe1	manager	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	active	2026-02-01 04:30:00+00	\N	d96195e2-0f16-f670-e070-101feaa5a668
92b0cc9d-1633-cbde-3356-6c31054360b9	cb68eae7-2955-5cae-e440-0a0190731fe1	tenant	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	active	2026-05-26 06:45:00+00	\N	ec42996c-0d3a-4576-f6f2-139d43c5841b
92b0cc9d-1633-cbde-3356-6c31054360b9	00000000-0000-0000-0000-000000000001	prospect	\N	active	2026-05-20 13:10:00+00	\N	922cff42-cd2b-b657-b28f-9c4c86bdb04c
638d9648-01dc-4225-a46d-11b804cae77a	00000000-0000-0000-0000-000000000001	prospect	\N	active	\N	\N	ac374d28-354c-4384-8da6-95814f7b4c79
638d9648-01dc-4225-a46d-11b804cae77a	cb68eae7-2955-5cae-e440-0a0190731fe1	tenant	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	active	2026-08-31 14:41:28.708912+00	\N	abd1ef33-d563-41bb-a4d2-4bc89fc79b0a
3d597748-da72-422a-b29d-3d104dd1ec9a	5d49310f-864e-4a65-926b-f4168d1dd9b6	owner	\N	active	\N	\N	a419e00a-e697-4299-8ec1-af7724daf668
0da807f6-32f9-42a3-b164-af11c7584e00	00000000-0000-0000-0000-000000000001	prospect	\N	active	\N	\N	5a211cdb-515e-4498-b5df-5221f3215b93
c956cebe-792a-4a40-81f4-e058c737d279	00000000-0000-0000-0000-000000000001	prospect	\N	active	\N	\N	ef0af1bf-8589-4add-8016-8f45be5fee25
d6e71466-8267-4a5c-86e1-4f940ee4c510	00000000-0000-0000-0000-000000000001	prospect	\N	active	\N	\N	1e6ffc8c-998c-4992-a029-717526df50c7
3e91a99c-7b18-48ae-a467-af9c8112acaf	00000000-0000-0000-0000-000000000001	prospect	\N	active	\N	\N	3c72974a-ef57-4748-bfdf-149ebcf85b4e
a3b13fff-6e12-48d5-b5d1-49c8ed34c7d0	00000000-0000-0000-0000-000000000001	prospect	\N	active	\N	\N	165319ed-ce63-47bf-b3f1-125d4741c349
cd97043e-49fe-48d6-bdfc-7ca1a0cb7964	00000000-0000-0000-0000-000000000001	prospect	\N	active	\N	\N	0699dc8f-01f0-4d2d-b938-ec6b70f376ea
1a7a2644-5f85-4057-b00b-445646f4f2a4	00000000-0000-0000-0000-000000000001	prospect	\N	active	\N	\N	5582192a-e5a8-4890-a387-127085283b2f
\.


--
-- Data for Name: residents; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.residents (id, org_id, branch_id, bed_id, full_name, phone, plan_kind, status, rent_paise, deposit_paise, user_id, moved_out_at, created_at, deleted_at, membership_id) FROM stdin;
2d924b68-0971-4d97-b1c8-cfbf53a6ea9a	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	fbac3c14-cb78-4324-b3dd-8d88e8faea39	Farah Ali	9000010006	monthly	active	950000	1900000	\N	\N	2026-08-11 08:43:11.52088+00	2026-08-12 17:28:45.087524+00	\N
33c7358d-48ab-456b-ae57-1e96dde1cfc6	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	af9dd177-134f-401c-b043-8ca067203d47	Eshan Shah	9000010005	monthly	active	950000	1900000	\N	\N	2026-08-11 08:43:11.52088+00	2026-08-12 17:28:45.087524+00	\N
6908b748-632f-4e44-8b82-e58028d0c4f7	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	41216d53-75b8-49a2-8b16-cb3882cd64c7	Bharat Singh	9000010002	monthly	active	950000	1900000	\N	\N	2026-08-11 08:43:11.52088+00	2026-08-12 17:28:45.087524+00	\N
1d0a045a-8f8b-4c38-95ae-276418f9523a	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	348fc045-19aa-481d-be5a-e68e0e9632e8	Chitra Rao	9000010003	monthly	active	950000	1900000	\N	\N	2026-08-11 08:43:11.52088+00	2026-08-12 17:28:45.087524+00	\N
7bbfd5ae-e59f-470c-b951-55be8a5a93a6	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	9ba09c1f-318d-41e1-9c27-f5e4ed463dc5	Asha Kumar	9000010001	monthly	active	850000	1700000	\N	\N	2026-08-11 08:43:11.52088+00	2026-08-12 17:28:45.087524+00	\N
c84df41a-fb81-4c5e-816f-d467b94e46b2	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	1aa3b446-a536-4717-8aeb-00933aa27868	Divya Nair	9000010004	monthly	active	850000	1700000	\N	\N	2026-08-11 08:43:11.52088+00	2026-08-12 17:28:45.087524+00	\N
1d996da5-a43c-4f1d-989a-818f7837c64a	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	3ce67705-76b3-4450-8c12-7f0e26a7c424	Bhargavraj	\N	monthly	active	1500000	3000000	1fd63177-2f43-4e85-997c-03314955bfca	\N	2026-08-12 12:00:39.478667+00	2026-08-12 17:28:45.087524+00	b2ad6579-9abd-4694-b9f0-6959590c6de3
b90bbc07-df8e-3bed-1d7d-7c245e23912a	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	ed724e26-59f9-9647-6a98-f39f40be3e2a	Krish	9000010103	monthly	active	1250000	2500000	92b0cc9d-1633-cbde-3356-6c31054360b9	\N	2026-06-01 05:30:00+00	\N	ec42996c-0d3a-4576-f6f2-139d43c5841b
f99f56ee-c00d-87b9-3503-a320594f36b8	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	\N	Naveen Rao	9000010104	monthly	moved_out	1200000	2400000	\N	2026-05-31 00:00:00+00	2026-01-15 04:30:00+00	\N	\N
2d83dfa8-338e-44da-bc3c-c864ac397d74	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	70010657-85cc-efaa-c365-3d5c22a4a8fe	Ashrith Bade	\N	monthly	active	1250000	2500000	638d9648-01dc-4225-a46d-11b804cae77a	\N	2026-08-31 14:41:28.708912+00	\N	abd1ef33-d563-41bb-a4d2-4bc89fc79b0a
\.


--
-- Data for Name: rooms; Type: TABLE DATA; Schema: pool; Owner: -
--

COPY pool.rooms (id, org_id, branch_id, number, cooling_type, created_at, deleted_at, floor_id) FROM stdin;
c877986d-c1a3-4591-b584-5b6e7268ef77	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	101	non_ac	2026-08-11 08:33:20.972622+00	\N	\N
361dea63-d579-4f90-b021-777558e8a38c	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	102	ac	2026-08-11 08:33:20.972622+00	\N	\N
6c580138-d35d-40c8-9f6e-94af09254f27	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	103	non_ac	2026-08-11 08:33:20.972622+00	\N	\N
12c562a7-5472-4fb2-8a8a-7f3113c05da0	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	201	ac	2026-08-11 08:33:20.972622+00	\N	\N
f1fc9814-e0f6-4797-844c-511b839f3660	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	202	non_ac	2026-08-11 08:33:20.972622+00	\N	\N
23dedd21-72b1-4b0b-b758-1129da3833ee	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	203	ac	2026-08-11 08:33:20.972622+00	\N	\N
1ef0757d-14f1-478d-b906-1f618e84564f	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	302	ac	2026-08-11 08:33:20.972622+00	\N	\N
a10297a6-cac6-40e6-84f3-e2e73bcd3cdc	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	303	non_ac	2026-08-11 08:33:20.972622+00	\N	\N
6b0c7dc0-8899-a1c8-cbed-9b4667ac33ea	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	103	non_ac	2026-01-05 04:10:00+00	\N	\N
6bf8fab9-d00e-e997-3883-cdc0c68e655a	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	201	ac	2026-01-05 04:10:00+00	\N	\N
d8b3aaea-ef8b-063b-a27a-9f14ff96bb13	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	202	non_ac	2026-01-05 04:10:00+00	\N	\N
61f0db3a-3817-98dc-48a4-d87f0db9ec5e	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	203	ac	2026-01-05 04:10:00+00	\N	\N
e3beab34-30cc-d1b5-b04d-2f85fdc035ba	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	101	non_ac	2026-01-05 04:10:00+00	\N	1114b103-e9e5-4ac6-af74-e02c5b62016c
e918a2e5-f700-12d1-5cd4-28f8a5340024	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	102	ac	2026-01-05 04:10:00+00	\N	e5c3ce85-2e17-446f-ab2d-4d824b1cefad
173e448b-004f-469e-9762-3a2a200eb5f5	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	301	non_ac	2026-08-11 08:33:20.972622+00	\N	\N
16f36aa2-c81f-46a4-9079-2c38b760eadb	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	301	ac	2026-09-02 06:50:19.750035+00	\N	57bb145c-0b47-4aa0-8b68-755267f4babb
e270d6af-009e-4d9a-94c9-243721c4c51b	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	302	non_ac	2026-09-02 06:50:19.750035+00	\N	57bb145c-0b47-4aa0-8b68-755267f4babb
36a45727-4adc-4fb9-a2a7-7996f67ed74a	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	401	non_ac	2026-09-02 08:33:20.141691+00	\N	86d5f4ad-be3d-4ab5-b300-4b0a3f3b75fc
993ce46e-f8ae-4a18-b835-984890c7b629	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	402	ac	2026-09-02 08:33:20.141691+00	\N	86d5f4ad-be3d-4ab5-b300-4b0a3f3b75fc
\.


--
-- Data for Name: device_tokens; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.device_tokens (id, user_id, org_id, token, platform, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: tenant_invitations; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.tenant_invitations (id, org_id, branch_id, intended_room_id, intended_bed_id, email, full_name, monthly_rent_paise, deposit_paise, planned_move_in_date, require_identity_verification, status, invited_by_user_id, accepted_by_user_id, expires_at, accepted_at, approved_at, approved_by_user_id, created_at, updated_at, last_sent_at, plan_kind) FROM stdin;
28402c57-a57e-4b0d-86a4-938643f2eeed	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	361dea63-d579-4f90-b021-777558e8a38c	2bff133d-5233-4d30-8810-6784ce020055	bhargav@gmail.com	Bhargav P	1500000	500000	2026-09-01	t	invited	1fd63177-2f43-4e85-997c-03314955bfca	\N	2026-08-19 11:24:26.345072+00	\N	\N	\N	2026-08-12 11:24:26.546763+00	2026-08-12 11:24:26.546763+00	2026-08-12 11:24:26.546763+00	monthly
23ca498f-624f-4e38-bf86-b32fcc40e56e	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	361dea63-d579-4f90-b021-777558e8a38c	2bff133d-5233-4d30-8810-6784ce020055	bhargav@gmail.com	Bhargav P	1500000	500000	2026-09-01	t	invited	1fd63177-2f43-4e85-997c-03314955bfca	\N	2026-08-19 11:24:39.549246+00	\N	\N	\N	2026-08-12 11:24:39.683454+00	2026-08-12 11:24:39.683454+00	2026-08-12 11:24:39.683454+00	monthly
d4c068c7-e064-464e-a17d-17b528fac313	32f33aef-dc6a-4caf-9f05-794f65f30c8a	af1202c3-9efe-4de3-a5ee-ce7383b81e0b	361dea63-d579-4f90-b021-777558e8a38c	2bff133d-5233-4d30-8810-6784ce020055	bhargav@gmail.com	Bhargav P	1500000	500000	2026-09-01	t	invited	1fd63177-2f43-4e85-997c-03314955bfca	\N	2026-08-19 11:26:21.311039+00	\N	\N	\N	2026-08-12 11:26:21.496597+00	2026-08-12 11:26:21.496597+00	2026-08-12 11:26:21.496597+00	monthly
1c596bb5-8095-84f5-a88e-3a8667629e64	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	e918a2e5-f700-12d1-5cd4-28f8a5340024	ed724e26-59f9-9647-6a98-f39f40be3e2a	kriahkunar2@gmail.com	Krish	1250000	2500000	2026-06-01	t	active	1fd63177-2f43-4e85-997c-03314955bfca	92b0cc9d-1633-cbde-3356-6c31054360b9	2026-05-29 04:30:00+00	2026-05-23 03:50:00+00	2026-05-26 06:45:00+00	1fd63177-2f43-4e85-997c-03314955bfca	2026-05-22 04:30:00+00	2026-05-26 06:45:00+00	2026-05-22 04:30:00+00	monthly
29464350-5477-47ff-4f38-a55c67db7fbb	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	e3beab34-30cc-d1b5-b04d-2f85fdc035ba	70010657-85cc-efaa-c365-3d5c22a4a8fe	sample.tenant@example.com	Sample Tenant	1150000	2300000	2026-09-01	t	invited	1fd63177-2f43-4e85-997c-03314955bfca	\N	2026-08-19 17:33:14.811877+00	\N	\N	\N	2026-08-11 17:28:45.087524+00	2026-08-11 17:33:14.811877+00	2026-08-11 17:33:14.811877+00	monthly
cd65a09d-61a1-49a7-b8b8-8ce2bb7d2661	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	e918a2e5-f700-12d1-5cd4-28f8a5340024	1bc87e0d-6590-4946-c659-fbd917e6edb0	test.owner@aalaya.in	Bhargavraj B	1100000	500000	2026-09-01	t	invited	1fd63177-2f43-4e85-997c-03314955bfca	\N	2026-08-19 17:43:09.337816+00	\N	\N	\N	2026-08-12 17:43:09.450083+00	2026-08-12 17:43:09.450083+00	2026-08-12 17:43:09.450083+00	monthly
ebef1e71-90dc-420d-8c25-7991cd78d08d	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	e3beab34-30cc-d1b5-b04d-2f85fdc035ba	70010657-85cc-efaa-c365-3d5c22a4a8fe	ashrithbade888@gmail.com	Ashrith Bade	1250000	2500000	\N	f	active	1fd63177-2f43-4e85-997c-03314955bfca	638d9648-01dc-4225-a46d-11b804cae77a	2026-09-07 14:41:23.027859+00	2026-08-31 14:41:27.859405+00	2026-08-31 14:41:28.708912+00	1fd63177-2f43-4e85-997c-03314955bfca	2026-08-31 14:41:23.141495+00	2026-08-31 14:41:28.708912+00	2026-08-31 14:41:23.141495+00	monthly
\.


--
-- Data for Name: digilocker_oauth_sessions; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.digilocker_oauth_sessions (id, invitation_id, user_id, state_hash, nonce_hash, pkce_verifier_ciphertext, expires_at, consumed_at, created_at) FROM stdin;
\.


--
-- Data for Name: email_otps; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.email_otps (id, user_id, email, purpose, code_hash, attempts, consumed_at, expires_at, created_at) FROM stdin;
43f36805-149e-47a4-a8f8-a3c920caa2b4	638d9648-01dc-4225-a46d-11b804cae77a	ashrithbade888@gmail.com	login	$argon2id$v=19$m=19456,t=2,p=1$Fxp+DeGsQx9WpjqLa0dpIA$PvnjOJAoC2ohOLyQYyifVOD6O1Wez6QszBjrBk2Cou8	0	2026-09-01 18:09:46.431791+00	2026-09-01 18:19:30.696177+00	2026-09-01 18:09:30.696177+00
3a32a147-1855-4e60-8d6d-8faa83e71510	a3b13fff-6e12-48d5-b5d1-49c8ed34c7d0	test.owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$OgK1W4Xh8SvLv2dHweA7wQ$VWCSoVPysr66fn9KrjTTp3/68Xycd6DNss0zBIZzwOQ	0	\N	2026-09-01 16:48:04.135733+00	2026-09-01 16:38:04.135733+00
35b89351-3aff-469a-8e44-428007bdd53c	cd97043e-49fe-48d6-bdfc-7ca1a0cb7964	tenant@hamsavilas.test	login	$argon2id$v=19$m=19456,t=2,p=1$ZFKI5AK+CrSc3woEdnxYHQ$P+86VwKaLMwo+BP9421SVo6pOM6FrWnsWp5bfqYROcg	0	\N	2026-09-01 17:49:09.917847+00	2026-09-01 17:39:09.917847+00
cb0ebfb1-6426-453b-ab8e-ef3683fc4b6b	cd97043e-49fe-48d6-bdfc-7ca1a0cb7964	tenant@hamsavilas.test	login	$argon2id$v=19$m=19456,t=2,p=1$4gRIHBQiAhchDMbFs11R/g$8Trv/UZ4SKcSpJuh8rbIlV8OfmA9tpigio0XrT/HOdY	0	\N	2026-09-01 17:49:33.291539+00	2026-09-01 17:39:33.291539+00
5e1482c5-ae47-4f64-8f60-caa351ecb691	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$DrvAIOp/bftcGRmWA2nwiQ$pzGOfN4VexJjjqgP+4znML0Sk+ioIl6SRD6JESnSxos	0	\N	2026-09-01 17:53:58.565828+00	2026-09-01 17:43:58.565828+00
b04d2427-8cd9-4545-a48b-4bd243042a58	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$ru85QKTpp1iI0ZVk3mdFXA$1sXn0+ZahuUxIrtRvqKipeIT3RJQh9m3pfdVqkhxpAA	0	2026-09-01 17:46:04.298675+00	2026-09-01 17:55:03.065589+00	2026-09-01 17:45:03.065589+00
35724a15-e206-4058-9eba-518d7795bc24	638d9648-01dc-4225-a46d-11b804cae77a	ashrithbade888@gmail.com	login	$argon2id$v=19$m=19456,t=2,p=1$1VYq9rB3+OUjCFyKucbAwQ$44md5hJtLbC7X24m2oN6wiRExL4txRv5MSgwHJUz7jQ	0	2026-09-01 17:47:49.828401+00	2026-09-01 17:57:33.185703+00	2026-09-01 17:47:33.185703+00
8c5eed0c-cc4f-4ae7-ae74-858850f2d3e8	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$rMne0gv6VTIcJwWqFaJnjA$+hrTWyZ1Dp4i4KNPZv+SVkQ8KrvvKnYX60ROriGEvls	0	2026-09-01 18:19:11.513675+00	2026-09-01 18:28:55.50306+00	2026-09-01 18:18:55.50306+00
fbf13f4f-50e6-4a82-beaf-d7a6ad7f98c5	0f2c58a3-c922-48ef-9571-68c5eb743a87	bhargavraj911@gmail.com	login	$argon2id$v=19$m=19456,t=2,p=1$O0h6RI2EC50IkBFVXLoclA$b5NPXKLFRiTW7TjuHkXad6Oq0ZsrAL0GKcS+qsK8WMk	0	2026-09-01 19:09:36.78746+00	2026-09-01 19:19:13.873148+00	2026-09-01 19:09:13.873148+00
909168a7-11c8-4e95-a7fe-482c372695bd	638d9648-01dc-4225-a46d-11b804cae77a	ashrithbade888@gmail.com	login	$argon2id$v=19$m=19456,t=2,p=1$EPXxpJTg04+ro4SRKCV7sA$Ou1odIsrHzViD68PUfa4Vyj69q+IbDGAp6xOh3hNwLg	0	2026-09-01 19:13:05.680157+00	2026-09-01 19:22:42.175287+00	2026-09-01 19:12:42.175287+00
baedc70c-9c30-4a6e-b466-b8557346f31c	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$ixol6xS+ylkk90m/VXRoXA$CuZF1+xceOZTQAl0XYd75qiZT74nmnIAQ2QgrADCk2o	0	2026-09-01 19:17:37.710073+00	2026-09-01 19:27:14.511915+00	2026-09-01 19:17:14.511915+00
a1af516a-73ea-4a94-8a0c-9fb3962d5e64	1a7a2644-5f85-4057-b00b-445646f4f2a4	explorer.test.0902@example.com	login	$argon2id$v=19$m=19456,t=2,p=1$7GSrXBGK91vxcLsx3SaALQ$6OPMjUjyGjnVHxAeTJBXPXeE48iVdQnkXKHQTK+HoDY	0	2026-09-01 19:28:36.25318+00	2026-09-01 19:38:00.103413+00	2026-09-01 19:28:00.103413+00
dd146932-78c2-4c5b-92fb-989e8cf4b5e0	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$AZxkumxO9UslhmfJkESYvQ$TrCojC1IxJGeFxsGb/3NvyyrRf8EjfhWh23mQ48T0cE	0	2026-09-01 19:47:29.810338+00	2026-09-01 19:57:26.584517+00	2026-09-01 19:47:26.584517+00
40a54d16-490d-4a6a-8aa4-468297aa35a9	0f2c58a3-c922-48ef-9571-68c5eb743a87	bhargavraj911@gmail.com	login	$argon2id$v=19$m=19456,t=2,p=1$7LnS63fJo1VotjRM4QDl8A$TCV2vOxfhXznTydywSzFJdC01DEolfIYo98AUQyXUFU	0	2026-09-01 19:48:26.126847+00	2026-09-01 19:58:22.904305+00	2026-09-01 19:48:22.904305+00
82dd749f-71a3-43e0-8c56-665235bde248	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$8M901VPNVKj/AvsHdGliyQ$aqnxKXTp/WAlSItyAN4XlKFXlBc47HTBmVcFzDQVpkg	0	2026-09-01 19:54:01.35563+00	2026-09-01 20:03:51.312327+00	2026-09-01 19:53:51.312327+00
deb94459-f32d-4295-b257-b3234f7d5186	638d9648-01dc-4225-a46d-11b804cae77a	ashrithbade888@gmail.com	login	$argon2id$v=19$m=19456,t=2,p=1$3lXqn3X7Nsx9V27WzcuIZw$+XvyNAnqIQwqv7BHROZH5rY5G4U3AyMLpqMglZ3JGXc	0	2026-09-01 20:04:58.908033+00	2026-09-01 20:14:40.286885+00	2026-09-01 20:04:40.286885+00
9c401341-6f4f-45d6-80b1-63634a17506a	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$bYXBbpo+Hk0m5hcGr6J8Xg$lOWw/6eirNhEewQ+Tbbzi9HBwN9ROTxKhwD1Tbvb1Fk	0	2026-09-01 20:08:20.427781+00	2026-09-01 20:18:17.155739+00	2026-09-01 20:08:17.155739+00
5e22c0be-2e1a-4491-a1a2-29cb970001ce	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$kpKuRofBlPruAorJz/o9vg$c/exXR7cyn9H7zORmEzTLR+g0nCnpm4+2oxjLb/eFGQ	0	2026-09-01 20:09:13.021321+00	2026-09-01 20:18:53.466219+00	2026-09-01 20:08:53.466219+00
a14f741f-0430-4cd7-ba41-b8f9254ce3b4	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$Qc1fSJO2iXcawHzfO0WVbA$ze9tsfuqzYnM2C9vO5dznH5VYdGtuo/YnTL3FO9hYP8	0	2026-09-02 03:23:27.326897+00	2026-09-02 03:33:24.092469+00	2026-09-02 03:23:24.092469+00
d6692a8c-062a-4173-98f5-b5216a2f0822	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$iCCmXnldPuzh442idqHW7g$jHDzWUZi6YM+FEx4JRdAKbyqehw5X6kn6oZ1Q/Y+shs	0	2026-09-02 03:27:43.937712+00	2026-09-02 03:37:40.688193+00	2026-09-02 03:27:40.688193+00
bbd8bef9-f4e6-4c2a-84d2-dce06512f8b2	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$L0L1aoQjzrn9Y6Xul0V6Bg$IG5Vz++5RI4gYw4NB2ktiqBR+vSdfgvvrbTJIYkPuYg	0	2026-09-02 03:29:33.914193+00	2026-09-02 03:39:13.638922+00	2026-09-02 03:29:13.638922+00
9fe0c608-eb39-48d0-812b-2f7966627561	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$drZz1oJriegbgpnPoRvxUw$Im4XkGilyHNabr4CF8jXGe/W/VkzLOXsCcdFSdUT/Yo	0	2026-09-02 03:52:51.276282+00	2026-09-02 04:02:47.9488+00	2026-09-02 03:52:47.9488+00
abe0647f-3589-453c-ba60-c0d82da2f294	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$MjPDRX0CwGUxllwOir7mRQ$ewqLijAVXArE13/yxxgVFpwTNhJwgxAxWjpNISz0c3I	0	2026-09-02 04:11:02.926991+00	2026-09-02 04:20:59.580041+00	2026-09-02 04:10:59.580041+00
51a74536-2d65-4136-b519-4d51f08c40e5	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$U94eIoq/KrdJXGK/iOGLUA$7FTiLGNmVSZkvj44ZpULiHc5ugG4r8+Hd17IMfWgLOM	0	2026-09-02 04:34:28.904504+00	2026-09-02 04:44:25.66701+00	2026-09-02 04:34:25.66701+00
8f5c9664-cc88-431f-815e-34dd9df2a1db	0f2c58a3-c922-48ef-9571-68c5eb743a87	bhargavraj911@gmail.com	login	$argon2id$v=19$m=19456,t=2,p=1$1JLwP40FS85IZ3cxbFWr1g$MqUM9mWO0UiVaH28gMnq+Vo3NOTKArN8+Yp/VuITR1s	0	2026-09-02 04:34:33.668317+00	2026-09-02 04:44:30.392443+00	2026-09-02 04:34:30.392443+00
f82da30a-54ba-41c4-a674-3df39dde8a9b	0f2c58a3-c922-48ef-9571-68c5eb743a87	bhargavraj911@gmail.com	login	$argon2id$v=19$m=19456,t=2,p=1$ShEEpcROYzzOzH2SzWfE4g$SylRTci1kVQZMj1RwpB7kFPFGlnpR0LTp/MGXOK/Kbw	0	2026-09-02 04:52:09.411011+00	2026-09-02 05:01:49.650834+00	2026-09-02 04:51:49.650834+00
ee4fed60-8cb5-471b-b3c6-a300e20a32cc	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$z1sgSIvgEpnJr9R+bkc/4g$LbxO2ILYeoJvHZOJ1TV4s2f1NePqPCDBbwaOK7jXVHs	0	2026-09-02 05:00:54.70571+00	2026-09-02 05:10:51.375917+00	2026-09-02 05:00:51.375917+00
33cd77f0-4981-4923-9f3d-d1c5173a8f10	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$mdkQV9ZzprA5hDK16Q7dVg$JnnjoQklhO/fISFjtwLARTGBVa4Al5rdjGjNTp/XJFE	0	2026-09-02 06:45:20.77406+00	2026-09-02 06:54:55.488577+00	2026-09-02 06:44:55.488577+00
f99ae1f5-4908-4a43-8159-1ac295e067f6	1fd63177-2f43-4e85-997c-03314955bfca	owner@aalaya.in	login	$argon2id$v=19$m=19456,t=2,p=1$m5OV+rIegMzagO3umeGVrg$BbcuO9stSGtk7G0FcJT6oOQMohOxUF8+nwL8BzD3XpA	0	2026-09-02 08:02:33.042389+00	2026-09-02 08:12:27.565391+00	2026-09-02 08:02:27.565391+00
\.


--
-- Data for Name: enquiries; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.enquiries (id, org_id, branch_id, prospect_id, phone, note, status, created_at) FROM stdin;
3ce74b3f-4073-798b-5dea-70ebf78e5c1f	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	c63ec3fb-6d34-4755-803f-7c078e20abef	9000010199	Is twin sharing available for two colleagues starting September?	contacted	2026-08-05 05:55:00+00
05e9ff9b-f750-9ca0-dfb8-203b8b174320	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	92b0cc9d-1633-cbde-3356-6c31054360b9	9000010103	Looking for a single AC bed from the first week of June. Do you have anything on the first floor?	contacted	2026-05-20 13:20:00+00
6ed8ba0d-76ec-f090-14a4-ea59c87def7d	cb68eae7-2955-5cae-e440-0a0190731fe1	bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	c63ec3fb-6d34-4755-803f-7c078e20abef	9000010199	Are all three meals included in the quoted rent, or is food billed separately?	contacted	2026-08-11 15:10:00+00
\.


--
-- Data for Name: invitation_identity_verifications; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.invitation_identity_verifications (id, invitation_id, provider, provider_reference, document_type, status, verified_at, safe_failure_reason, created_at) FROM stdin;
d77f05c5-c33f-265c-d09f-37d85e58677b	1c596bb5-8095-84f5-a88e-3a8667629e64	digilocker	DL-REF-8842013	AADHAAR	failed	\N	name on the document did not match the invitation	2026-05-24 09:35:00+00
2596c567-5a22-47dd-f379-1d4d0b5f4db5	1c596bb5-8095-84f5-a88e-3a8667629e64	manual	MANUAL-2026-0525-01	AADHAAR	verified	2026-05-25 06:00:00+00	\N	2026-05-25 06:00:00+00
\.


--
-- Data for Name: listing_vacancy; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.listing_vacancy (branch_id, org_id, beds_total, beds_available, starting_price_paise, updated_at, stale) FROM stdin;
bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	cb68eae7-2955-5cae-e440-0a0190731fe1	24	22	1250000	2026-09-02 17:02:52.307112+00	f
\.


--
-- Data for Name: manual_kyc_reviews; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.manual_kyc_reviews (id, invitation_id, reviewer_user_id, decision, reason, request_id, created_at) FROM stdin;
5f3e3573-c296-0fd9-6674-26c5aab597e8	1c596bb5-8095-84f5-a88e-3a8667629e64	1fd63177-2f43-4e85-997c-03314955bfca	approved	Aadhaar and rental agreement checked in person at the property on 25 May 2026; the mismatch was an initial-versus-full-name difference.	8d59f91e-e7da-7899-e475-d77f7c8d1be3	2026-05-25 06:00:00+00
\.


--
-- Data for Name: otp_codes; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.otp_codes (id, user_id, phone, code_hash, attempts, consumed_at, expires_at, created_at) FROM stdin;
7bc44efe-2c1b-4229-820b-5b35f93fafa5	00e1e1f4-96d0-43ec-b27e-e5cc04297ffa	9000012345	edb8882229e6ea7d57a87ee4b3953b3fb133631430a6917bab320657bc69046c	0	\N	2026-08-11 08:04:18.505772+00	2026-08-11 07:59:18.505772+00
2858ff60-e7e9-4f71-a3e3-b09740e279e1	00e1e1f4-96d0-43ec-b27e-e5cc04297ffa	9000012345	617ea49fdafd4bb5a99ce2dedbab8e83fafc854fbeed083f6277ec45085c9a72	0	2026-08-11 08:08:11.128445+00	2026-08-11 08:12:56.433626+00	2026-08-11 08:07:56.433626+00
3c29545e-dd96-4242-bac9-e6955c117c27	00e1e1f4-96d0-43ec-b27e-e5cc04297ffa	9000012345	2829cf14248cdc78e5372803ebf213f179a5ffc782fbf15fe49f5d96a431d2fa	0	2026-08-11 08:28:26.578915+00	2026-08-11 08:33:10.891199+00	2026-08-11 08:28:10.891199+00
\.


--
-- Data for Name: public_listings; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.public_listings (branch_id, org_id, title, area, city, lat, lng, description, amenities, status, rejection_note, review_request, published_at, created_at, updated_at) FROM stdin;
bdbfddf5-eb26-9e23-5fd4-2ba1307460a4	cb68eae7-2955-5cae-e440-0a0190731fe1	Hamsa Vilas PG - HSR Layout	HSR Layout	Bengaluru	12.9121	77.6446	Fully furnished co-living close to HSR Sector 2 bus stop and the Agara lake loop. Single, twin and triple sharing across AC and non-AC rooms, with three meals a day included.	WiFi, Three meals (veg), Laundry, Power backup, CCTV, Hot water, Housekeeping, Refrigerator, Study desk	live	\N	\N	2026-01-20 06:30:00+00	2026-01-18 09:30:00+00	2026-08-12 17:33:14.811877+00
\.


--
-- Data for Name: receipt_templates; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.receipt_templates (org_id, display_name, address, footer, accent, logo_key, updated_at) FROM stdin;
cb68eae7-2955-5cae-e440-0a0190731fe1	Hamsa Vilas PG	#42, 27th Main Road, HSR Layout Sector 2, Bengaluru, Karnataka 560102	Rent is payable on or before the 5th of each month; a late fee of INR 100 per day applies thereafter.\nPayments are adjusted against the earliest outstanding dues first.\nReport any discrepancy within 7 days at +91 90000 10101; thereafter this invoice is deemed accepted.	#194FC2	\N	2026-09-02 03:52:17.856159+00
\.


--
-- Data for Name: refresh_tokens; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.refresh_tokens (token_hash, user_id, org_id, expires_at, created_at, revoked_at) FROM stdin;
20c12f51d637bd35ea7619044e53e16569107d3bb4b4a0533dbe7df4b74e3cbc	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 17:27:16.546518+00	2026-08-31 17:27:16.546518+00	2026-08-31 17:52:35.553612+00
3c414f6efec6f7be4af7448f8048519f1e52f28b8a2bd8484c290307216b0719	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 14:28:22.448225+00	2026-08-31 14:28:22.448225+00	2026-08-31 14:42:06.347953+00
1ddd51836d9f1da097241494083be37f5140fb3564adc8b3cd25f207226a8d72	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 14:33:48.391503+00	2026-08-31 14:33:48.391503+00	2026-08-31 14:42:06.347953+00
82b2bb4a4defcadc50a557f1d1947cca42e19433e026b9b38a9e2b73abba4de2	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 17:52:35.827209+00	2026-08-31 17:52:35.827209+00	2026-08-31 18:11:46.634218+00
83a016fc17063dff41f489388f53213db7dbf0857f49419c90428e18c0ee9e77	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 14:41:22.752753+00	2026-08-31 14:41:22.752753+00	2026-08-31 14:42:06.347953+00
e24794f0210303ea98e2b5b5a002eeb3b7cff669753d88ee3144b07856c46aa0	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 18:11:46.774364+00	2026-08-31 18:11:46.774364+00	2026-09-01 06:02:04.407401+00
d638f2e49b52349facf24e07ff9f695ade8449bd7a9dc745150e8babeea9e799	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 06:02:05.277149+00	2026-09-01 06:02:05.277149+00	2026-09-01 06:22:09.365218+00
76eb785888df90f1786fb6a7c6b99c74f829b5fccddfae3275f17bd5d1bb2f23	92b0cc9d-1633-cbde-3356-6c31054360b9	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 15:13:49.264778+00	2026-08-31 15:13:49.264778+00	\N
13141e83368f04958e6eeff090abefbcc04a089e42894565ba57a388d3e2b128	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 06:22:09.516416+00	2026-09-01 06:22:09.516416+00	\N
3e32964fd8826a6cb0a8e348e7ccbedd49745cde7d34d1ce1c7af5cf2af90107	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 06:53:16.080258+00	2026-09-01 06:53:16.080258+00	2026-09-01 07:16:25.956067+00
14bfbc4d0a23e9301ab7a19cab3961ab05746ec61e1c65ac6dfd4810beb13ac7	638d9648-01dc-4225-a46d-11b804cae77a	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 15:07:58.908864+00	2026-08-31 15:07:58.908864+00	2026-08-31 15:50:12.705062+00
46ab5b539cc9bf08ecac10841075679d44418d68c066afa01a33c5496b294734	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 07:16:26.214788+00	2026-09-01 07:16:26.214788+00	\N
780daee90367667b130cb3fdcae7c88fb8ea7ac6af2eb52083f734341fc267f0	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 07:29:35.40167+00	2026-09-01 07:29:35.40167+00	2026-09-01 07:47:22.852568+00
1fdc1470852c11a5fd752e0c56b4c06b477b0d1ac323670c4c1e7a448325c67b	638d9648-01dc-4225-a46d-11b804cae77a	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 14:42:35.931529+00	2026-08-31 14:42:35.931529+00	2026-08-31 15:51:19.687592+00
2f0b9d8d20e83c62dfa4280760170c4b7d43212911110cd292fd23cc422ebabe	638d9648-01dc-4225-a46d-11b804cae77a	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 15:50:12.843091+00	2026-08-31 15:50:12.843091+00	2026-08-31 15:51:19.687592+00
b5d8b86fad635bf600c77a6ae4262ff40f528e525e5c8141caddb3efb8448992	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 07:47:23.001012+00	2026-09-01 07:47:23.001012+00	\N
bc6a9ca3ff579defcc5cc606db92030e54f5d434c082cdb79a237e082f5541a5	638d9648-01dc-4225-a46d-11b804cae77a	00000000-0000-0000-0000-000000000001	2026-09-14 14:41:27.254584+00	2026-08-31 14:41:27.254584+00	2026-08-31 15:51:19.687592+00
771d4a6861012519e382fa325208dabaedb0a348ecc307a7c9dd28480f77fb3c	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 07:51:11.994866+00	2026-09-01 07:51:11.994866+00	2026-09-01 08:22:26.162259+00
4240bdb837b8367bd93475b2a95fd28d332141bb9b5316543b7ccd354471e477	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 15:13:43.941492+00	2026-08-31 15:13:43.941492+00	2026-08-31 16:04:56.907786+00
489cc3d6f8dde263826944129c75e5da7f013d1c4762cdaf26e2a4b99d0d15d8	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 15:51:55.892011+00	2026-08-31 15:51:55.892011+00	2026-08-31 16:04:56.907786+00
471028392cc510365fcc2c34f04920ad9c1cb65eb6f5663c95dbce43fc6525ac	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 08:22:26.695888+00	2026-09-01 08:22:26.695888+00	2026-09-01 10:05:18.521754+00
2d27576ba72adc104d0a4a711b6bb4a52f12ba74fd19347cd4ff9b5aefc5a8e8	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 10:05:18.699156+00	2026-09-01 10:05:18.699156+00	\N
99cb888873ec2c5bed6bb9fe640bc99718e3143392468105fb232be9721a845e	638d9648-01dc-4225-a46d-11b804cae77a	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 16:05:28.673951+00	2026-08-31 16:05:28.673951+00	2026-08-31 16:09:22.402076+00
6ca4a532d891db791d6c406f1eb97d914bea0660ba6087c43de2c43d2b541c70	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 11:42:26.884061+00	2026-09-01 11:42:26.884061+00	2026-09-01 12:48:16.050914+00
1e3a718e3f4841cf1644c7e72d86ca448a3fc5d99b0ba80163fa9da9fd4c5470	0f2c58a3-c922-48ef-9571-68c5eb743a87	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 16:09:39.585753+00	2026-08-31 16:09:39.585753+00	2026-08-31 16:10:28.02204+00
23f0dc946b3df52724ad820671ec622ab2234f99cfe7985220ebba4fbdbcf003	0f2c58a3-c922-48ef-9571-68c5eb743a87	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 14:28:26.82042+00	2026-08-31 14:28:26.82042+00	2026-08-31 16:10:28.02204+00
be392dbaf4acd430aad9ff17559526c6c072fbac375711ed2b21ad81042a5735	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 12:48:16.24615+00	2026-09-01 12:48:16.24615+00	2026-09-01 13:08:07.094308+00
db3d489d72b95293d27632b46ee08baf20f9781d6ab767fe4532d490b48632ef	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 13:08:07.32947+00	2026-09-01 13:08:07.32947+00	\N
9e706b274aed89bffbed4efeb573edc36f5de332fffb9734adb68bbb243f78c3	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 15:02:11.191638+00	2026-09-01 15:02:11.191638+00	2026-09-01 15:22:35.91117+00
fb27d3271f0b7c55a0e4ff5ae3864bff5bdfbe76eb60f13de6fbe03acddae93c	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 15:22:36.082376+00	2026-09-01 15:22:36.082376+00	2026-09-01 15:46:02.934299+00
25a41e2f6c73eec639f62c4afdb04c4f8ad0f23b042c4a856b23d9e89f73629b	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 16:27:01.755366+00	2026-08-31 16:27:01.755366+00	\N
c71eabdcbd71e0ce0d1d3c5e9b40084eb9a5b7779e6ecf88807e2e3395b228fc	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 15:46:03.090759+00	2026-09-01 15:46:03.090759+00	\N
3f460ea087e58838c225a84d2b29037245803374df8600c0164465db26a6f0f1	0f2c58a3-c922-48ef-9571-68c5eb743a87	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 16:27:05.532474+00	2026-08-31 16:27:05.532474+00	\N
53073ee761f9ebc1602c6f2a5c18c07a7ff889b2af23baf8b25b0a17fd250c8d	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 16:04:05.051936+00	2026-09-01 16:04:05.051936+00	\N
85e713f352b6a4d9f55cd329bdee32faf554276f7c6818e79c76aa1314b45969	c63ec3fb-6d34-4755-803f-7c078e20abef	00000000-0000-0000-0000-000000000001	2026-09-14 16:27:09.153925+00	2026-08-31 16:27:09.153925+00	\N
4a1adc5d75a37406cdd86921d874778bdc8b696c613c85e9834b7554cbad10b0	3e91a99c-7b18-48ae-a467-af9c8112acaf	00000000-0000-0000-0000-000000000001	2026-09-15 16:13:25.808764+00	2026-09-01 16:13:25.808764+00	\N
302912c673625bc01e877a3098be6269ed5573c0ce3216f3fb4a7b1aecf3135e	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 16:28:03.174666+00	2026-08-31 16:28:03.174666+00	\N
e7d5a3906c737681433a945a69c631a606d9f697e09578bfe9d0e5ed0226edef	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 16:27:59.976561+00	2026-09-01 16:27:59.976561+00	\N
2f51a06b4328cdd18a0b48f9a370743ba50f8fbc5bbc5ef2f093e6c89ff3dbac	0f2c58a3-c922-48ef-9571-68c5eb743a87	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 16:28:19.633214+00	2026-08-31 16:28:19.633214+00	\N
bdf977a4801978a506b78db30e3081e5560e3d98c02c49ebf578a105b9d0a861	a3b13fff-6e12-48d5-b5d1-49c8ed34c7d0	00000000-0000-0000-0000-000000000001	2026-09-15 16:38:58.816649+00	2026-09-01 16:38:58.816649+00	\N
76404821ac4c750d458ddd7c073198fee0dd6fc8ee49c1dbfffbf1b669c79165	c63ec3fb-6d34-4755-803f-7c078e20abef	00000000-0000-0000-0000-000000000001	2026-09-14 16:28:24.231298+00	2026-08-31 16:28:24.231298+00	\N
bf9ab596eaad0a0109c9af1e9642937d76de4ee58a6939df087e853a68234da3	0f2c58a3-c922-48ef-9571-68c5eb743a87	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 16:39:56.58+00	2026-09-01 16:39:56.58+00	2026-09-01 17:00:31.949919+00
32fb01c6e23da8c553be9c13b4769d20bfa1f39d14c1ffd359aba3870f21cb65	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 17:07:45.931251+00	2026-08-31 17:07:45.931251+00	2026-08-31 17:27:16.29329+00
f33872971766a79e6da9633a94f9a286020d3669c94939fec51daeaf4a08dd1f	0f2c58a3-c922-48ef-9571-68c5eb743a87	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 17:00:32.078579+00	2026-09-01 17:00:32.078579+00	2026-09-01 17:18:10.639823+00
d3a6a94063250643463f74c5defb3b964b87a732e5ff492b5a9369bfaf3ada13	0f2c58a3-c922-48ef-9571-68c5eb743a87	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 17:18:10.799578+00	2026-09-01 17:18:10.799578+00	\N
bef8c725b2f33c9cd6e31cc8996b57341938cc5139befe2046b39ccc7b97d701	92b0cc9d-1633-cbde-3356-6c31054360b9	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 17:21:04.460138+00	2026-09-01 17:21:04.460138+00	\N
f6d159b5df048392db1b32b5a155a1af4e713d041a30bbafae7ae91040bf8921	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 17:46:05.027872+00	2026-09-01 17:46:05.027872+00	\N
938576dec4d84a028bc6b17ee4a7c738a9c90ad9bae82bc9192048ca8b5c01d3	0f2c58a3-c922-48ef-9571-68c5eb743a87	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 19:09:37.764234+00	2026-09-01 19:09:37.764234+00	\N
7e10c56925cf42336b3d61edb6e15d30e5fb559f3bd97aa2b695beddf3c61ad8	92b0cc9d-1633-cbde-3356-6c31054360b9	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-08-29 22:32:18.387543+00	2026-08-15 22:32:18.387543+00	\N
8e7343f4c1c8394467d13cbc2c446fd4488c239248f35d133450243b1f6fa041	92b0cc9d-1633-cbde-3356-6c31054360b9	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 14:30:17.585112+00	2026-08-31 14:30:17.585112+00	\N
eda743e488bc43cb8f4ccebc25274705dd1444c75ad752dfc9d127446066e0f2	638d9648-01dc-4225-a46d-11b804cae77a	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-14 16:18:09.593051+00	2026-08-31 16:18:09.593051+00	2026-09-01 17:54:04.056037+00
916284b9f9528c029b077e0769df656ded70ceb31c780a2b5bc4d4c09b313d2b	638d9648-01dc-4225-a46d-11b804cae77a	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 17:47:50.591297+00	2026-09-01 17:47:50.591297+00	2026-09-01 17:54:04.056037+00
23afc32fb6ea3bcfc679a02979a0fc196e0959b0f57e30893b844b870006365a	638d9648-01dc-4225-a46d-11b804cae77a	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 18:09:47.181373+00	2026-09-01 18:09:47.181373+00	\N
7c5b6dc03465a7f9cf48bca6252e9ec3b23683692e08f50a74101e91b5770455	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 18:19:12.283005+00	2026-09-01 18:19:12.283005+00	2026-09-01 18:40:15.802429+00
d3c30fc1337ed39e8c88f8c22a2be3e3c8546e14fc16b8d2defe2b1a463038b8	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 18:40:15.974875+00	2026-09-01 18:40:15.974875+00	2026-09-01 18:58:50.529115+00
ce286b0005ae4bbb125112e06037d5f22fdf49e7cd51e74ed66bed7bd2982948	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 18:58:50.686032+00	2026-09-01 18:58:50.686032+00	\N
e71ba2ac52e2a9b6e02734cf441aca84127508e11fd4e7a828bf9d2469788fde	638d9648-01dc-4225-a46d-11b804cae77a	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 19:13:06.661211+00	2026-09-01 19:13:06.661211+00	\N
5884d89531e9d3f26fc1696bd668063fb616c04f86a477a9a19032b225ec55a9	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 19:17:38.45465+00	2026-09-01 19:17:38.45465+00	\N
0bdb3afd8b28b2484657929066d1f1a03c056327d0a3d5f1a6b1baa2b370f3c8	1a7a2644-5f85-4057-b00b-445646f4f2a4	00000000-0000-0000-0000-000000000001	2026-09-15 19:28:37.105473+00	2026-09-01 19:28:37.105473+00	\N
c07557508b0593a665b3e0772fdb79fe1916853836d3312b466c14b3eb5650b5	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 19:47:30.568844+00	2026-09-01 19:47:30.568844+00	\N
483ca1f525c8d582fefc9377b0a0f499fd4aaf7abdc06c03527a03343a2a11e7	0f2c58a3-c922-48ef-9571-68c5eb743a87	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 19:48:26.6874+00	2026-09-01 19:48:26.6874+00	\N
ed734114684171758657001fe5e30121eceaccb39644a6d9da5554ed9670f10f	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 19:54:01.917454+00	2026-09-01 19:54:01.917454+00	\N
9af0b77f733209e6b9f37d4a212b7aa2966f272263e63d88b2e1421481cd52d4	638d9648-01dc-4225-a46d-11b804cae77a	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 20:04:59.635692+00	2026-09-01 20:04:59.635692+00	\N
5886dbc72c9270ccbec07efc20592bcd8e2db1d67bde4d0b97b6d4a93f4232c7	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 20:08:21.201349+00	2026-09-01 20:08:21.201349+00	\N
f9a01513701d5f63fbfc024e3ffc7b611c034425159f343f9d2b591386f8e98f	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-15 20:09:13.767329+00	2026-09-01 20:09:13.767329+00	\N
1cbc53365932fb8442a514d10df26ff95035596e95a54542af9b29aa91440bbb	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 03:23:28.14393+00	2026-09-02 03:23:28.14393+00	\N
0b75328dd56040b9a8b9f7d6d3f40fcdabc248bae4721e92459722dd80397b6e	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 03:27:44.850575+00	2026-09-02 03:27:44.850575+00	\N
e097f14af81fa6fd5582e2a051ae605394825709cdb2ebaa887c2eaffb60f87c	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 03:52:52.029164+00	2026-09-02 03:52:52.029164+00	\N
872c95f35c5007b56ff026c510c8912242d4eb3ab0600c39531ff595ee71753a	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 03:29:34.576625+00	2026-09-02 03:29:34.576625+00	2026-09-02 03:57:15.347678+00
e39bda7c6f6bebe1d23c0095bde71617584ed110dff8018d0d2cfb7fbb85cdf1	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 04:11:03.693942+00	2026-09-02 04:11:03.693942+00	\N
38d869b8304ee073532432899945a49265843274b00d50a15ba2d3c95b1af48f	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 03:57:15.495668+00	2026-09-02 03:57:15.495668+00	2026-09-02 04:22:23.842058+00
c985d93668fc0faa80e5826e665062fa91f9c57ef1c37dc8b97cd3e25e77ac07	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 04:22:23.999432+00	2026-09-02 04:22:23.999432+00	\N
359c7da427bb7d5212ba639738519f7b3dcce580b9ae6a43ea4dcb6779ddf76d	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 04:34:29.673685+00	2026-09-02 04:34:29.673685+00	\N
8407694d8abc448bc4e56112f0b8c1ecdcd13fb8f3a3ddca833472f88e25a420	0f2c58a3-c922-48ef-9571-68c5eb743a87	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 04:34:34.449964+00	2026-09-02 04:34:34.449964+00	\N
e46056f9dc05899ff06efda1c2def31fc837d9d90e70dbbcf61e91ae6b2950d3	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 05:00:55.510955+00	2026-09-02 05:00:55.510955+00	\N
d00349f08275d9904b9ed93db30d8bcabe267678e1ebf31b64f2fe4c8463cdfe	0f2c58a3-c922-48ef-9571-68c5eb743a87	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 04:52:10.18619+00	2026-09-02 04:52:10.18619+00	2026-09-02 05:08:15.165815+00
8bb479ba5a0d09a892f668b9977fddf519c5aab37b9c2cde0fd1ff986233a784	0f2c58a3-c922-48ef-9571-68c5eb743a87	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 05:08:15.276044+00	2026-09-02 05:08:15.276044+00	2026-09-02 05:28:11.680701+00
d3b061c28a85f0f01e8b7a116a7f7d9c695856c91eb32e4b6e5f23e6795315d6	0f2c58a3-c922-48ef-9571-68c5eb743a87	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 05:28:11.850695+00	2026-09-02 05:28:11.850695+00	2026-09-02 05:50:09.128793+00
949bbe996002cb5fb0f73f1ace335db10bd0054b7d70a2d81ee2989832346c03	0f2c58a3-c922-48ef-9571-68c5eb743a87	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 05:50:09.229342+00	2026-09-02 05:50:09.229342+00	2026-09-02 06:06:36.727317+00
715102eae3d679324c3696291a5b99e81d7d22348df8637cbc801d5944f7dbb1	0f2c58a3-c922-48ef-9571-68c5eb743a87	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 06:06:36.910262+00	2026-09-02 06:06:36.910262+00	\N
91ef3a333e5e3b24fe3dd75005b10866822f0b1991d2ac509d772091786aeb6c	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 08:02:33.88243+00	2026-09-02 08:02:33.88243+00	\N
d601df4e0e62256028cb64a11af88a2bdad1e847a03654dc33702c4d5ab624ea	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 06:45:21.991201+00	2026-09-02 06:45:21.991201+00	2026-09-02 08:26:10.195394+00
b59cb28bd197fc22dacd0de66d89d73f02e4bc16bbdbdc9eadefe6d20a0098ea	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 08:26:10.409916+00	2026-09-02 08:26:10.409916+00	2026-09-02 08:45:35.805228+00
a54c6a0e7f862f6d3fdd912d37d833c3a37594bd9fcdee89cabb784196952a13	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 08:45:35.963851+00	2026-09-02 08:45:35.963851+00	2026-09-02 09:02:28.330941+00
9a694c9c8b8429a87ddc4e4c6a1b2d74baad5198394418c10236d46ae1d6a1ad	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 09:02:28.50679+00	2026-09-02 09:02:28.50679+00	2026-09-02 09:22:29.291588+00
90e21ddf50b562ce060179b4403d62d467d2d2816a1c055adf1891b95366f0d5	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 09:22:29.499556+00	2026-09-02 09:22:29.499556+00	2026-09-02 09:40:56.843936+00
046a044dfbb3c6d40180dcaab259376f2f97c8c5f351acd053313135fa859d2c	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 09:40:57.03428+00	2026-09-02 09:40:57.03428+00	2026-09-02 10:00:06.85101+00
3b23b106090f8640db77185d095eaa7dd26ba5f41a3543580dcc232f6e1ca7ec	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 10:00:06.994738+00	2026-09-02 10:00:06.994738+00	2026-09-02 16:54:31.017636+00
6c387f05b27029e1eda617f9a129db30a36ee716e79e46845c626dc4bd8dfba0	1fd63177-2f43-4e85-997c-03314955bfca	cb68eae7-2955-5cae-e440-0a0190731fe1	2026-09-16 16:54:31.17653+00	2026-09-02 16:54:31.17653+00	\N
\.


--
-- Data for Name: schema_migrations_tenant; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.schema_migrations_tenant (schema_name, version, applied_at) FROM stdin;
public	0001_init	2026-08-11 07:17:33.023001+00
pool	0001_init	2026-08-11 07:17:33.023001+00
public	0002_email_auth	2026-08-12 07:11:36.180244+00
public	0003_tenant_invitations	2026-08-12 07:11:36.371755+00
public	0004_membership_status	2026-08-12 07:11:36.550365+00
public	0005_invitation_constraints	2026-08-12 07:11:36.743297+00
public	0006_invitation_delivery	2026-08-12 07:11:36.906713+00
public	0007_membership_identifiers	2026-08-12 07:11:37.115307+00
public	0008_invitation_kyc	2026-08-12 07:11:37.524597+00
pool	0002_membership_links	2026-08-12 07:11:37.8676+00
public	0009_platform_organization	2026-08-12 09:22:54.928342+00
public	0010_invitation_plan_kind	2026-08-12 11:21:32.223166+00
public	0011_invitation_daily_plan	2026-08-12 11:21:32.436538+00
pool	0003_invoice_numbers	2026-08-12 16:48:29.979868+00
pool	0005_floors	2026-09-01 19:46:55.336951+00
pool	0006_invoice_seq_per_org	2026-09-02 04:59:50.565024+00
\.


--
-- Data for Name: tenant_placement; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.tenant_placement (org_id, mode, schema_name, status, updated_at) FROM stdin;
32f33aef-dc6a-4caf-9f05-794f65f30c8a	pooled	pool	active	2026-08-11 07:57:22.986182+00
00000000-0000-0000-0000-000000000001	pooled	pool	active	2026-08-12 09:22:54.820454+00
cb68eae7-2955-5cae-e440-0a0190731fe1	pooled	pool	active	2026-08-12 17:33:14.811877+00
5d49310f-864e-4a65-926b-f4168d1dd9b6	pooled	pool	active	2026-09-01 06:21:52.556512+00
\.


--
-- Data for Name: trusted_devices; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.trusted_devices (id, user_id, device_id, name, created_at, last_seen_at) FROM stdin;
d16125ab-afef-4781-ba1e-7af32dfc65a6	00e1e1f4-96d0-43ec-b27e-e5cc04297ffa	ac3e2cf1-f9e9-4973-bf89-95f945fcc631	sdk_gphone64_arm64	2026-08-11 08:28:26.912928+00	2026-08-11 08:28:26.912928+00
\.


--
-- Data for Name: verifications; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.verifications (id, user_id, org_id, method, status, verified_name, phone, emergency_contact, reason, created_at) FROM stdin;
abb24493-899a-1a72-b82f-7b71ee67e911	92b0cc9d-1633-cbde-3356-6c31054360b9	cb68eae7-2955-5cae-e440-0a0190731fe1	manual	verified	Krish	9000010103	9000010104	Name on the Aadhaar matched after in-person review	2026-05-25 06:00:00+00
\.


--
-- Name: audit_log_id_seq; Type: SEQUENCE SET; Schema: pool; Owner: -
--

SELECT pg_catalog.setval('pool.audit_log_id_seq', 139, true);


--
-- Name: complaint_events_id_seq; Type: SEQUENCE SET; Schema: pool; Owner: -
--

SELECT pg_catalog.setval('pool.complaint_events_id_seq', 18, true);


--
-- PostgreSQL database dump complete
--

\unrestrict J0cMhaf3dsyG0jO2Duoi5ekR4bma3oi0z1obBIPxCTARl0cC9JGhocZbJHblaLN

