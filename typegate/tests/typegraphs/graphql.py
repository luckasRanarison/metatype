from typegraph.graphs.typegraph import TypeGraph
from typegraph.materializers.graphql import GraphQLRuntime
from typegraph.policies import allow_all
from typegraph.types import types as t

with TypeGraph("graphql") as g:
    gql = GraphQLRuntime("https://example.com/api/graphql")

    user = t.struct(
        {
            "id": t.integer(),
            "email": t.string(),
            "name": t.string(),
        }
    ).named("User")

    user_by_id = gql.query(t.struct({"id": t.integer()}), g("User")).add_policy(
        allow_all()
    )
    update_user = gql.mutation(
        t.struct(
            {
                "id": t.integer(),
                "patch": t.struct(
                    {
                        "name": t.string().optional(),
                        "email": t.string().optional(),
                    }
                ),  # TODO min_props: 1
            }
        ),
        g("User"),
    ).add_policy(allow_all())

    g.expose(user=user_by_id, updateUser=update_user)
