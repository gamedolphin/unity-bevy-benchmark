using UnityEngine;
using Unity.Entities;

public class SpawnerAuthoring : MonoBehaviour
{
    public int count;
    public float maxSize;
    public GameObject item;
    public GameObject robot;
    public float robotSpeed;
}

public struct SpawnerComponent : IComponentData
{
    public int count;
    public float maxSize;
    public Entity item;
    public Entity robot;
    public float robotSpeed;
}

public class SpawnerBaking : Baker<SpawnerAuthoring>
{
    public override void Bake(SpawnerAuthoring authoring)
    {
        var itemPrefab = GetEntity(authoring.item, TransformUsageFlags.Dynamic);
        var robotPrefab = GetEntity(authoring.robot, TransformUsageFlags.Dynamic);


        var entity = GetEntity(TransformUsageFlags.Dynamic);
        AddComponent(entity, new SpawnerComponent()
        {
            item = itemPrefab,
            robot = robotPrefab,
            maxSize = authoring.maxSize,
            count = authoring.count,
            robotSpeed = authoring.robotSpeed,
        });
    }
}
